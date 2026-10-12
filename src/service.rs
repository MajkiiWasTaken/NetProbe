/************************************************
* File: service.rs
* Author: Michal Švrček
*
* Network service diagnostics for SSH, HTTP,
* HTTPS, Modbus TCP and MQTT
*
* ver. 0.3.0
*************************************************/

use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

pub struct ServiceResult {
    pub description: String,
    pub details: Vec<String>,
}

pub fn check(
    kind: &str,
    host: &str,
    port: Option<u16>,
    timeout: Duration,
    unit_id: u8,
    insecure: bool,
) -> Result<ServiceResult, String> {
    if host.trim().is_empty() {
        return Err("Host cannot be empty".into());
    }

    if insecure && !kind.eq_ignore_ascii_case("https") {
        return Err("--insecure is only supported for HTTPS".into());
    }

    match kind.to_ascii_lowercase().as_str() {
        "ssh" => ssh(host, port.unwrap_or(22), timeout),
        "http" => http(host, port.unwrap_or(80), false, timeout, false),
        "https" => http(host, port.unwrap_or(443), true, timeout, insecure),
        "modbus" => modbus(host, port.unwrap_or(502), timeout, unit_id),
        "mqtt" => mqtt(host, port.unwrap_or(1883), timeout),
        _ => Err(format!("Unsupported service: {kind}")),
    }
}

fn connect(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, String> {
    let addresses: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolution failed: {e}"))?
        .collect();

    if addresses.is_empty() {
        return Err("No IP addresses resolved".into());
    }

    let mut last_error = String::new();

    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|e| e.to_string())?;

                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|e| e.to_string())?;

                return Ok(stream);
            }
            Err(e) => last_error = e.to_string(),
        }
    }

    Err(format!("TCP connection failed: {last_error}"))
}

fn ssh(host: &str, port: u16, timeout: Duration) -> Result<ServiceResult, String> {
    let mut stream = connect(host, port, timeout)?;

    let mut banner = Vec::new();
    let mut byte = [0u8; 1];

    // SSH servers may send a few informational lines first.
    for _ in 0..2048 {
        stream
            .read_exact(&mut byte)
            .map_err(|e| format!("SSH banner read failed: {e}"))?;

        banner.push(byte[0]);

        if byte[0] == b'\n' {
            let line = String::from_utf8_lossy(&banner).trim().to_string();

            if line.starts_with("SSH-") {
                return Ok(ServiceResult {
                    description: "SSH service identified".into(),
                    details: vec![format!("Address: {host}:{port}"), format!("Banner: {line}")],
                });
            }

            banner.clear();
        }
    }

    Err("SSH identification banner not found".into())
}

fn http(
    host: &str,
    port: u16,
    secure: bool,
    timeout: Duration,
    insecure: bool,
) -> Result<ServiceResult, String> {
    let scheme = if secure { "https" } else { "http" };

    // Brackets are required around IPv6 addresses in URLs.
    let url_host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };

    let url = format!("{scheme}://{url_host}:{port}/");

    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(insecure)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(&url).send().map_err(|e| {
        let mut message = format!("{scheme} request failed: {e}");
        let mut source = std::error::Error::source(&e);

        while let Some(cause) = source {
            message.push_str(&format!("\n  Caused by: {cause}"));
            source = cause.source();
        }

        message
    })?;

    let status = response.status();

    let server = response
        .headers()
        .get(reqwest::header::SERVER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    let redirect = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let mut details = vec![
        format!("URL: {url}"),
        format!("HTTP status: {}", status.as_u16()),
        format!("Server: {server}"),
    ];

    if let Some(location) = redirect {
        details.push(format!("Redirect: {location}"));
    }

    if insecure {
        details.push("TLS certificate verification: DISABLED".into());
    }

    Ok(ServiceResult {
        description: format!("{} service responded", scheme.to_ascii_uppercase()),
        details,
    })
}

fn modbus(host: &str, port: u16, timeout: Duration, unit_id: u8) -> Result<ServiceResult, String> {
    let mut stream = connect(host, port, timeout)?;

    // Modbus TCP:
    // Transaction ID: 0x4E50
    // Protocol ID: 0
    // Unit ID: 1
    // Function 43/14: Read Device Identification
    let request: [u8; 11] = [
        0x4E, 0x50, 0x00, 0x00, 0x00, 0x05, unit_id, 0x2B, 0x0E, 0x01, 0x00,
    ];

    stream
        .write_all(&request)
        .map_err(|e| format!("Modbus request failed: {e}"))?;

    let mut header = [0u8; 7];

    stream
        .read_exact(&mut header)
        .map_err(|e| format!("Modbus response header failed: {e}"))?;

    let transaction = u16::from_be_bytes([header[0], header[1]]);
    let protocol = u16::from_be_bytes([header[2], header[3]]);
    let length = u16::from_be_bytes([header[4], header[5]]) as usize;

    if transaction != 0x4E50 || protocol != 0 {
        return Err("Invalid Modbus TCP header".into());
    }

    // MBAP length includes Unit ID, which was already read.
    if !(2..=254).contains(&length) {
        return Err("Invalid Modbus response length".into());
    }

    if header[6] != unit_id {
        return Err(format!(
            "Unexpected Modbus Unit ID: expected {unit_id}, received {}",
            header[6]
        ));
    }

    let mut pdu = vec![0u8; length - 1];

    stream
        .read_exact(&mut pdu)
        .map_err(|e| format!("Modbus response body failed: {e}"))?;

    let description = match pdu.first().copied() {
        Some(0x2B) if pdu.get(1) == Some(&0x0E) => "Modbus device identification supported",
        Some(0xAB) => "Modbus exception response received",
        _ => return Err("Unexpected Modbus function code".into()),
    };

    let mut details = vec![
        format!("Address: {host}:{port}"),
        format!("Unit ID: {}", header[6]),
    ];

    if pdu.first() == Some(&0xAB) {
        if let Some(code) = pdu.get(1) {
            details.push(format!("Exception code: 0x{code:02X}"));
        }
    }

    Ok(ServiceResult {
        description: description.into(),
        details,
    })
}

fn mqtt(host: &str, port: u16, timeout: Duration) -> Result<ServiceResult, String> {
    let mut stream = connect(host, port, timeout)?;

    // MQTT 3.1.1 CONNECT, clean session, client ID "nprobe".
    // No username, password or publish operation.
    let packet: [u8; 20] = [
        0x10, 0x12, 0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04, 0x02, 0x00, 0x0A, 0x00, 0x06, b'n',
        b'p', b'r', b'o', b'b', b'e',
    ];

    stream
        .write_all(&packet)
        .map_err(|e| format!("MQTT CONNECT failed: {e}"))?;

    let mut response = [0u8; 4];

    stream
        .read_exact(&mut response)
        .map_err(|e| format!("MQTT CONNACK failed: {e}"))?;

    if response[0] != 0x20 || response[1] != 0x02 || response[2] & 0xFE != 0 {
        return Err("Invalid MQTT CONNACK packet".into());
    }

    let status = match response[3] {
        0 => "Connection accepted",
        1 => "Unacceptable protocol version",
        2 => "Identifier rejected",
        3 => "Server unavailable",
        4 => "Bad username or password",
        5 => "Not authorized",
        _ => return Err("Invalid MQTT return code".into()),
    };

    Ok(ServiceResult {
        description: "MQTT broker responded".into(),
        details: vec![
            format!("Address: {host}:{port}"),
            format!("CONNACK: {status}"),
        ],
    })
}
