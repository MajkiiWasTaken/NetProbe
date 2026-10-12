/************************************************
* File: monitor.rs
* Author: Michal Švrček
*
* TCP availability monitoring, latency measurement,
* outage detection and JSON history export.
*
* ver. 0.4.0
*************************************************/

use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use std::{
    fs,
    net::{TcpStream, ToSocketAddrs},
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crate::output;

static STOP: OnceLock<Arc<AtomicBool>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub struct Measurement {
    pub timestamp: DateTime<Utc>,
    pub reachable: bool,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MonitorReport {
    pub host: String,
    pub port: u16,
    pub started: DateTime<Utc>,
    pub finished: DateTime<Utc>,
    pub interval_seconds: u64,
    pub timeout_ms: u64,
    pub total_checks: usize,
    pub successful_checks: usize,
    pub failed_checks: usize,
    pub outages: usize,
    pub availability_percent: f64,
    pub average_latency_ms: Option<f64>,
    pub min_latency_ms: Option<f64>,
    pub max_latency_ms: Option<f64>,
    pub measurements: Vec<Measurement>,
}

pub struct MonitorOptions<'a> {
    pub host: &'a str,
    pub port: u16,
    pub interval: u64,
    pub timeout_ms: u64,
    pub count: Option<u64>,
    pub json: Option<&'a Path>,
}

fn stop_flag() -> Result<Arc<AtomicBool>, String> {
    let flag = STOP
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone();

    flag.store(false, Ordering::SeqCst);

    static HANDLER: OnceLock<Result<(), String>> = OnceLock::new();

    let result = HANDLER.get_or_init(|| {
        let flag = flag.clone();

        ctrlc::set_handler(move || {
            flag.store(true, Ordering::SeqCst);
        })
        .map_err(|e| format!("Cannot install Ctrl+C handler: {e}"))
    });

    result.clone()?;
    Ok(flag)
}

fn tcp_check(host: &str, port: u16, timeout: Duration) -> Result<f64, String> {
    let start = Instant::now();

    let addresses: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS: {e}"))?
        .collect();

    if addresses.is_empty() {
        return Err("No IP addresses resolved".into());
    }

    let mut last_error = String::from("Connection failed");

    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                drop(stream);
                return Ok(ms);
            }
            Err(error) => {
                last_error = error.to_string();
            }
        }
    }

    Err(last_error)
}

fn interruptible_sleep(duration: Duration, stop: &AtomicBool) {
    let start = Instant::now();

    while !stop.load(Ordering::SeqCst) {
        let remaining = duration.saturating_sub(start.elapsed());

        if remaining.is_zero() {
            break;
        }

        thread::sleep(remaining.min(Duration::from_millis(100)));
    }
}

fn save_json(path: &Path, report: &MonitorReport) -> Result<(), String> {
    let content =
        serde_json::to_vec_pretty(report).map_err(|e| format!("JSON serialization failed: {e}"))?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create output directory: {e}"))?;
        }
    }

    fs::write(path, content).map_err(|e| format!("Cannot write {}: {e}", path.display()))
}

pub fn run(options: MonitorOptions<'_>) -> Result<(), String> {
    if options.host.trim().is_empty() {
        return Err("Host cannot be empty".into());
    }

    if options.port == 0 {
        return Err("Port must be between 1 and 65535".into());
    }

    if !(1..=3600).contains(&options.interval) {
        return Err("Interval must be between 1 and 3600 seconds".into());
    }

    if !(1..=30_000).contains(&options.timeout_ms) {
        return Err("Timeout must be between 1 and 30000 ms".into());
    }

    if options.count == Some(0) {
        return Err("Count must be greater than zero".into());
    }

    let stop = stop_flag()?;
    let started = Utc::now();

    let mut measurements = Vec::new();
    let mut outages = 0usize;
    let mut previous_up: Option<bool> = None;

    output::heading(&format!("Monitoring: {}:{}", options.host, options.port));

    output::info(&format!(
        "Interval: {}s | Timeout: {}ms",
        options.interval, options.timeout_ms
    ));

    output::info("Press Ctrl+C to stop monitoring.");
    println!();

    let interval = Duration::from_secs(options.interval);
    let timeout = Duration::from_millis(options.timeout_ms);

    while !stop.load(Ordering::SeqCst) {
        let iteration_start = Instant::now();
        let timestamp = Utc::now();

        let result = tcp_check(options.host, options.port, timeout);
        let reachable = result.is_ok();

        let time_label = timestamp.with_timezone(&Local).format("%H:%M:%S");

        let (latency_ms, error) = match result {
            Ok(ms) => {
                output::ok(&format!("{time_label}  UP      {ms:.2} ms"));

                (Some(ms), None)
            }
            Err(error) => {
                output::warn(&format!("{time_label}  DOWN    {error}"));

                (None, Some(error))
            }
        };

        if !reachable && previous_up != Some(false) {
            outages += 1;

            if previous_up == Some(true) {
                output::warn("Device became unavailable");
            }
        }

        if reachable && previous_up == Some(false) {
            output::ok("Device recovered");
        }

        previous_up = Some(reachable);

        measurements.push(Measurement {
            timestamp,
            reachable,
            latency_ms,
            error,
        });

        if let Some(limit) = options.count {
            if measurements.len() as u64 >= limit {
                break;
            }
        }

        // Start-to-start scheduling:
        // subtract the time already used by the TCP probe.
        interruptible_sleep(interval.saturating_sub(iteration_start.elapsed()), &stop);
    }

    let total = measurements.len();

    let successful = measurements.iter().filter(|m| m.reachable).count();

    let failed = total - successful;

    let latencies: Vec<f64> = measurements.iter().filter_map(|m| m.latency_ms).collect();

    let average_latency = if latencies.is_empty() {
        None
    } else {
        Some(latencies.iter().sum::<f64>() / latencies.len() as f64)
    };

    let min_latency = latencies.iter().copied().reduce(f64::min);
    let max_latency = latencies.iter().copied().reduce(f64::max);

    let availability = if total == 0 {
        0.0
    } else {
        successful as f64 * 100.0 / total as f64
    };

    let report = MonitorReport {
        host: options.host.to_string(),
        port: options.port,
        started,
        finished: Utc::now(),
        interval_seconds: options.interval,
        timeout_ms: options.timeout_ms,
        total_checks: total,
        successful_checks: successful,
        failed_checks: failed,
        outages,
        availability_percent: availability,
        average_latency_ms: average_latency,
        min_latency_ms: min_latency,
        max_latency_ms: max_latency,
        measurements,
    };

    println!();
    output::heading("Monitoring summary");

    output::info(&format!(
        "Checks: {} | Successful: {} | Failed: {}",
        report.total_checks, report.successful_checks, report.failed_checks
    ));

    output::info(&format!(
        "Availability: {:.1}% | Outages: {}",
        report.availability_percent, report.outages
    ));

    if let Some(avg) = report.average_latency_ms {
        output::info(&format!(
            "TCP latency: avg {:.2} ms | min {:.2} ms | max {:.2} ms",
            avg,
            report.min_latency_ms.unwrap_or(avg),
            report.max_latency_ms.unwrap_or(avg)
        ));
    }

    if let Some(path) = options.json {
        save_json(path, &report)?;

        output::ok(&format!("JSON history saved: {}", path.display()));
    }

    Ok(())
}
