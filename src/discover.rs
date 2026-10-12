/************************************************
* File: discover.rs
* Author: Michal Švrček
*
* Parallel IPv4 network discovery using TCP probes
*
* ver. 0.2.0
*************************************************/

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct HostResult {
    pub ip: Ipv4Addr,
    pub open_ports: Vec<u16>,
}

pub struct DiscoveryResult {
    pub scanned: usize,
    pub elapsed: Duration,
    pub hosts: Vec<HostResult>,
}

pub fn parse_subnet(cidr: &str) -> Result<Vec<Ipv4Addr>, String> {
    let (address, prefix) = cidr
        .split_once('/')
        .ok_or("Expected IPv4 CIDR, for example 192.168.1.0/24")?;

    let ip: Ipv4Addr = address.parse().map_err(|_| "Invalid IPv4 address")?;

    let prefix: u32 = prefix.parse().map_err(|_| "Invalid CIDR prefix")?;

    if prefix > 32 {
        return Err("CIDR prefix must be between 0 and 32".into());
    }

    let value = u32::from(ip);

    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };

    let network = value & mask;
    let size = 1u64 << (32 - prefix);

    if size > 256 {
        return Err("Discovery is limited to 256 addresses. \
             Use a /24 or smaller subnet."
            .into());
    }

    // Reject scans targeting public or otherwise unsupported IP ranges.
    for offset in 0..size {
        let candidate = Ipv4Addr::from(network + offset as u32);

        if !(candidate.is_private() || candidate.is_loopback() || candidate.is_link_local()) {
            return Err(format!(
                "Address {candidate} is outside the allowed local ranges"
            ));
        }
    }

    // For conventional subnets, skip network and broadcast addresses.
    let (start, end) = if prefix <= 30 {
        (1, size - 1)
    } else {
        (0, size)
    };

    let mut addresses = Vec::new();

    for offset in start..end {
        addresses.push(Ipv4Addr::from(network + offset as u32));
    }

    Ok(addresses)
}

fn probe_host(ip: Ipv4Addr, ports: &[u16], timeout: Duration) -> HostResult {
    let mut open_ports = Vec::new();

    for &port in ports {
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, port));

        if let Ok(stream) = TcpStream::connect_timeout(&addr, timeout) {
            open_ports.push(port);
            drop(stream);
        }
    }

    HostResult { ip, open_ports }
}

pub fn discover(
    subnet: &str,
    ports: &[u16],
    timeout: Duration,
    threads: usize,
) -> Result<DiscoveryResult, String> {
    if threads == 0 || threads > 64 {
        return Err("Worker count must be between 1 and 64".into());
    }

    if ports.is_empty() || ports.len() > 32 {
        return Err("Specify between 1 and 32 TCP ports".into());
    }

    let addresses = parse_subnet(subnet)?;
    let scanned = addresses.len();
    let start = Instant::now();

    let mut hosts = Vec::new();

    // Process addresses in batches, never spawning more than
    // the configured number of workers simultaneously.
    for batch in addresses.chunks(threads) {
        let results = thread::scope(|scope| {
            let handles: Vec<_> = batch
                .iter()
                .map(|&ip| scope.spawn(move || probe_host(ip, ports, timeout)))
                .collect();

            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| "Discovery worker panicked".to_string())
                })
                .collect::<Result<Vec<_>, _>>()
        })?;

        hosts.extend(
            results
                .into_iter()
                .filter(|host| !host.open_ports.is_empty()),
        );
    }

    hosts.sort_by_key(|host| u32::from(host.ip));

    Ok(DiscoveryResult {
        scanned,
        elapsed: start.elapsed(),
        hosts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_private_subnet() {
        let addresses = parse_subnet("192.168.1.0/30").unwrap();

        assert_eq!(
            addresses,
            vec![Ipv4Addr::new(192, 168, 1, 1), Ipv4Addr::new(192, 168, 1, 2),]
        );
    }

    #[test]
    fn reject_large_subnet() {
        assert!(parse_subnet("192.168.0.0/16").is_err());
    }

    #[test]
    fn reject_public_subnet() {
        assert!(parse_subnet("8.8.8.0/24").is_err());
    }

    #[test]
    fn single_host() {
        let addresses = parse_subnet("127.0.0.1/32").unwrap();

        assert_eq!(addresses, vec![Ipv4Addr::LOCALHOST]);
    }

    #[test]
    fn reject_invalid_prefix() {
        assert!(parse_subnet("192.168.1.0/33").is_err());
    }
}
