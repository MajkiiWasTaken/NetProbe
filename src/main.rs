/************************************************
* File: main.rs
* Author: Michal Švrček
*
* NetProbe command line interface
*
* ver. 0.1.0
*************************************************/
mod net;
mod output;
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
