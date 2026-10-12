/************************************************
* File: config.rs
* Author: Michal Švrček
*
* TOML device configuration management.
*
* ver. 0.5.0
*************************************************/

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, fs, path::PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub devices: BTreeMap<String, Device>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub host: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub services: BTreeMap<String, Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub port: u16,

    #[serde(default)]
    pub insecure: bool,
}

pub fn config_path() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let base = env::var_os("APPDATA").ok_or("APPDATA is not defined")?;

        Ok(PathBuf::from(base).join("NetProbe").join("config.toml"))
    }

    #[cfg(not(windows))]
    {
        let base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .ok_or("Cannot find user configuration directory")?;

        Ok(base.join("netprobe").join("config.toml"))
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;

        toml::from_str(&content).map_err(|e| format!("Invalid config.toml: {e}"))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = config_path()?;

        let directory = path.parent().ok_or("Invalid configuration path")?;

        fs::create_dir_all(directory).map_err(|e| e.to_string())?;

        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;

        // Prepare new contents first, then replace the file.
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, content).map_err(|e| e.to_string())?;

        // Preserve the previous configuration before replacing it.
        if path.exists() {
            let backup = path.with_extension("toml.bak");
            fs::copy(&path, &backup).map_err(|e| format!("Config backup failed: {e}"))?;

            // Windows rename does not replace an existing file.
            // This is not an atomic replacement on Windows.
            if let Err(error) = fs::remove_file(&path) {
                let _ = fs::remove_file(&temporary);
                return Err(format!("Cannot replace config: {error}"));
            }
        }

        fs::rename(&temporary, &path).map_err(|e| {
            format!(
                "Cannot save config: {e}. Recovery copy: {}",
                path.with_extension("toml.bak").display()
            )
        })
    }

    pub fn resolve_host(&self, name_or_host: &str) -> String {
        self.devices
            .get(name_or_host)
            .map(|device| device.host.clone())
            .unwrap_or_else(|| name_or_host.to_string())
    }

    pub fn find_device(&self, name: &str) -> Result<&Device, String> {
        self.devices
            .get(name)
            .ok_or_else(|| format!("Device '{name}' not found"))
    }
}
