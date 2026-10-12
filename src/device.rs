/************************************************
* File: device.rs
* Author: Michal Švrček
*
* Device profile commands and configuration.
*
* ver. 0.5.0
*************************************************/

use crate::{
    config::{Config, Device, Service},
    output,
};
use std::collections::BTreeMap;

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

pub fn add(name: &str, host: &str, description: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err("Device name may only contain letters, digits, '-' and '_'".into());
    }

    if host.trim().is_empty() || host.chars().any(char::is_whitespace) {
        return Err("Invalid host address".into());
    }

    let mut config = Config::load()?;

    if config.devices.contains_key(name) {
        return Err(format!("Device '{name}' already exists"));
    }

    let mut services = BTreeMap::new();

    services.insert(
        "ssh".into(),
        Service {
            port: 22,
            insecure: false,
        },
    );

    services.insert(
        "https".into(),
        Service {
            port: 443,
            insecure: false,
        },
    );

    config.devices.insert(
        name.to_string(),
        Device {
            host: host.to_string(),
            description: description.to_string(),
            services,
        },
    );

    config.save()?;

    output::ok(&format!("Device '{name}' added"));
    output::info(&format!("Host: {host}"));

    Ok(())
}

pub fn list() -> Result<(), String> {
    let config = Config::load()?;

    output::heading("Configured devices");

    if config.devices.is_empty() {
        output::warn("No devices configured");
        return Ok(());
    }

    for (name, device) in &config.devices {
        output::ok(&format!("{name:<20} {}", device.host));
    }

    println!();
    output::info(&format!("{} device(s) configured", config.devices.len()));

    Ok(())
}

pub fn info(name: &str) -> Result<(), String> {
    let config = Config::load()?;
    let device = config.find_device(name)?;

    output::heading(&format!("Device: {name}"));
    output::info(&format!("Host: {}", device.host));

    if !device.description.is_empty() {
        output::info(&format!("Description: {}", device.description));
    }

    println!();
    output::heading("Services");

    for (name, service) in &device.services {
        output::info(&format!("{name:<12} TCP {}", service.port));
    }

    Ok(())
}

pub fn remove(name: &str, yes: bool) -> Result<(), String> {
    if !yes {
        return Err("Use --yes to confirm device removal".into());
    }

    let mut config = Config::load()?;

    if config.devices.remove(name).is_none() {
        return Err(format!("Device '{name}' not found"));
    }

    config.save()?;

    output::ok(&format!("Device '{name}' removed"));
    Ok(())
}

pub fn resolve_monitor_target(
    name_or_host: &str,
    service_name: Option<&str>,
    explicit_port: Option<u16>,
) -> Result<(String, u16), String> {
    let config = Config::load()?;

    if let Some(device) = config.devices.get(name_or_host) {
        let port = match (explicit_port, service_name) {
            (Some(port), _) => port,
            (None, Some(name)) => {
                device
                    .services
                    .get(name)
                    .ok_or_else(|| format!("Service '{name}' not configured for '{name_or_host}'"))?
                    .port
            }
            (None, None) => device
                .services
                .get("ssh")
                .map(|service| service.port)
                .unwrap_or(22),
        };

        if port == 0 {
            return Err("TCP port cannot be zero".into());
        }

        return Ok((device.host.clone(), port));
    }

    if service_name.is_some() {
        return Err("--service requires a configured device profile".into());
    }

    let port = explicit_port.unwrap_or(22);

    if port == 0 {
        return Err("TCP port cannot be zero".into());
    }

    Ok((name_or_host.to_string(), port))
}
