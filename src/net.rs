/************************************************
* File: net.rs
* Author: Michal Švrček
*
* DNS, ICMP and TCP connectivity probes
*
* ver. 0.1.0
*************************************************/
use std::{
    collections::BTreeSet,
    net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs},
    process::Command,
    time::{Duration, Instant},
};

pub fn resolve(host: &str) -> Result<Vec<IpAddr>, String> {
    let addresses = (host, 0)
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolution failed for '{host}': {e}"))?;
    let ips: BTreeSet<IpAddr> = addresses.map(|addr| addr.ip()).collect();
    if ips.is_empty() {
        return Err("No IP addresses found".into());
    }
    Ok(ips.into_iter().collect())
}

pub fn tcp(host: &str, port: u16, timeout: Duration) -> Result<(SocketAddr, Duration), String> {
    let addresses: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("Cannot resolve {host}: {e}"))?
        .collect();
    if addresses.is_empty() {
        return Err("No resolved addresses".into());
    }
    let mut last_error = String::from("No address accepted the connection");
    for addr in addresses {
        let start = Instant::now();
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => {
                drop(stream);
                return Ok((addr, start.elapsed()));
            }
            Err(e) => last_error = format!("{addr}: {e}"),
        }
    }
    Err(last_error)
}

pub fn ping(host: &str, count: u8, timeout: Duration) -> Result<bool, String> {
    if host.is_empty() || host.starts_with('-') {
        return Err("Invalid host".into());
    }
    let mut command = Command::new("ping");
    #[cfg(windows)]
    {
        command.args([
            "-n",
            &count.to_string(),
            "-w",
            &timeout.as_millis().min(u32::MAX as u128).to_string(),
            host,
        ]);
    }
    #[cfg(not(windows))]
    {
        command.args([
            "-c",
            &count.to_string(),
            "-W",
            &timeout.as_secs().max(1).to_string(),
            "--",
            host,
        ]);
    }
    let status = command
        .status()
        .map_err(|e| format!("Cannot launch ping: {e}"))?;
    Ok(status.success())
}

pub fn parse_ports(spec: &str) -> Result<Vec<u16>, String> {
    let mut ports = BTreeSet::new();
    for token in spec.split(',') {
        let token = token.trim();
        if token.is_empty() {
            return Err("Empty port entry".into());
        }
        if let Some((start, end)) = token.split_once('-') {
            let a: u16 = start
                .parse()
                .map_err(|_| format!("Invalid port: {start}"))?;
            let b: u16 = end.parse().map_err(|_| format!("Invalid port: {end}"))?;
            if a == 0 || b < a {
                return Err(format!("Invalid port range: {token}"));
            }
            if usize::from(b - a) + 1 > 256 {
                return Err("Maximum 256 ports per scan".into());
            }
            for port in a..=b {
                ports.insert(port);
            }
        } else {
            let port: u16 = token
                .parse()
                .map_err(|_| format!("Invalid port: {token}"))?;
            if port == 0 {
                return Err("Port 0 is not a valid TCP destination".into());
            }
            ports.insert(port);
        }
        if ports.len() > 256 {
            return Err("Maximum 256 ports per scan".into());
        }
    }
    Ok(ports.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ports() {
        assert_eq!(
            parse_ports("22,80,80,443-444").unwrap(),
            vec![22, 80, 443, 444]
        );
    }
    #[test]
    fn invalid_ports() {
        assert!(parse_ports("0").is_err());
        assert!(parse_ports("100-10").is_err());
        assert!(parse_ports("1-300").is_err());
    }
}
