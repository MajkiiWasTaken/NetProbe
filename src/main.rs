/************************************************
* File: main.rs
* Author: Michal Švrček
*
* NetProbe command line interface
*
* ver. 0.2.0
*************************************************/

mod discover;
mod net;
mod output;
mod service;

use clap::{Parser, Subcommand};
use std::{process::ExitCode, time::Duration};

#[derive(Parser)]
#[command(
    version,
    about = "Cross-platform network diagnostics",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Resolve a hostname to IP addresses
    Dns { host: String },
    /// Run system ping
    Ping {
        host: String,
        #[arg(short, long, default_value_t = 4, value_parser = clap::value_parser!(u8).range(1..=20))]
        count: u8,
        #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(1..=30000))]
        timeout_ms: u64,
    },
    /// Test one TCP port
    Tcp {
        host: String,
        port: u16,
        #[arg(long, default_value_t = 1500, value_parser = clap::value_parser!(u64).range(1..=30000))]
        timeout_ms: u64,
    },
    /// Check up to 256 TCP ports on a host
    Scan {
        host: String,
        #[arg(long, default_value = "22,80,443,502,1883,8080")]
        ports: String,
        #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u64).range(1..=30000))]
        timeout_ms: u64,
    },
    /// Discover TCP-responsive hosts in a local IPv4 subnet
    Discover {
        /// Local IPv4 subnet, e.g. 192.168.1.0/24
        subnet: String,

        /// TCP ports used for host detection
        #[arg(long, default_value = "22,80,443,502,1883")]
        ports: String,

        /// Timeout per TCP connection in milliseconds
        #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..=10000))]
        timeout_ms: u64,

        /// Maximum number of concurrent workers
        #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u8).range(1..=64))]
        threads: u8,
    },
    /// Diagnose an SSH, HTTP, HTTPS, Modbus TCP or MQTT service
    Service {
        /// Protocol: SSH, HTTP, HTPPS, MODBUS TCP, MQTT
        kind: String,

        /// Hostname or IP
        host: String,

        /// Custom TCP port
        #[arg(long)]
        port: Option<u16>,

        /// Connection timeout val in millis
        #[arg(long, default_value_t = 3000)]
        timeout_ms: u64,

        /// Modbus TCP Unit ID
        #[arg(long, default_value_t = 1)]
        unit_id: u8,

        /// Disable HTTPS certificate verification (diagnostics only)
        #[arg(long)]
        insecure: bool,
    },
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    output::banner();
    match cli.command {
        Commands::Dns { host } => {
            output::heading(&format!("DNS: {host}"));
            for ip in net::resolve(&host)? {
                output::ok(&ip.to_string());
            }
        }
        Commands::Ping {
            host,
            count,
            timeout_ms,
        } => {
            output::heading(&format!("Ping: {host}"));
            let alive = net::ping(&host, count, Duration::from_millis(timeout_ms))?;
            if alive {
                output::ok("Host responded");
            } else {
                return Err("Ping failed or ICMP is blocked".into());
            }
        }
        Commands::Tcp {
            host,
            port,
            timeout_ms,
        } => {
            output::heading(&format!("TCP: {host}:{port}"));
            let (addr, elapsed) = net::tcp(&host, port, Duration::from_millis(timeout_ms))?;
            output::ok(&format!(
                "Connected to {addr} in {} ms",
                elapsed.as_millis()
            ));
        }
        Commands::Scan {
            host,
            ports,
            timeout_ms,
        } => {
            let ports = net::parse_ports(&ports)?;
            output::heading(&format!("TCP scan: {host} ({} ports)", ports.len()));
            output::info("Sequential TCP connectivity checks");
            let mut open = 0;
            for port in &ports {
                match net::tcp(&host, *port, Duration::from_millis(timeout_ms)) {
                    Ok((addr, elapsed)) => {
                        output::ok(&format!(
                            "{port:<5} reachable  {addr}  {}ms",
                            elapsed.as_millis()
                        ));
                        open += 1;
                    }
                    Err(_) => output::warn(&format!("{port:<5} closed / filtered")),
                }
            }
            output::info(&format!("Checked {} ports, {open} reachable", ports.len()));
        }
        Commands::Discover {
            subnet,
            ports,
            timeout_ms,
            threads,
        } => {
            let ports = net::parse_ports(&ports)?;

            output::heading(&format!("Network discovery: {subnet}"));

            let addresses = discover::parse_subnet(&subnet)?;

            output::info(&format!(
                "Scanning {} addresses using {} workers",
                addresses.len(),
                threads
            ));

            output::info(&format!(
                "TCP ports: {}",
                ports
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));

            let result = discover::discover(
                &subnet,
                &ports,
                Duration::from_millis(timeout_ms),
                threads as usize,
            )?;

            println!();

            for host in &result.hosts {
                let port_list = host
                    .open_ports
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(",");

                output::ok(&format!("{:<16} TCP {}", host.ip, port_list));
            }

            println!();

            output::info(&format!(
                "Scanned {} addresses, {} responsive hosts, {:.2}s",
                result.scanned,
                result.hosts.len(),
                result.elapsed.as_secs_f64()
            ));

            if result.hosts.is_empty() {
                output::warn("No TCP-responsive hosts found on the selected ports.");
            }
        }

        Commands::Service {
            kind,
            host,
            port,
            timeout_ms,
            unit_id,
            insecure,
        } => {
            if !(1..=30_000).contains(&timeout_ms) {
                return Err("Timeout must be between 1 and 30000 ms".into());
            }

            output::heading(&format!(
                "{} diagnostics: {}",
                kind.to_ascii_uppercase(),
                host
            ));

            let result = service::check(
                &kind,
                &host,
                port,
                Duration::from_millis(timeout_ms),
                unit_id,
                insecure,
            )?;

            for detail in &result.details {
                output::info(detail);
            }

            output::ok(&result.description);
        }
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            output::error(&error);
            ExitCode::FAILURE
        }
    }
}
