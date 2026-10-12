# NetProbe

**NetProbe** is a lightweight, cross-platform network diagnostics CLI written in Rust. Designed for developers and administrators to troubleshoot networks, Raspberry Pi devices, Linux servers, and TCP services.

**Platforms:** Windows & Linux | **License:** MIT

## Features

- DNS resolution and ICMP ping
- TCP connectivity checks and port scanning
- Parallel IPv4 network discovery
- SSH, HTTP/HTTPS, Modbus TCP, and MQTT diagnostics
- Continuous TCP monitoring with latency and outage detection
- JSON monitoring reports
- TOML-based device profiles
- Colored terminal output

## Installation

Download the latest package from [GitHub Releases](https://github.com/MajkiiWasTaken/NetProbe/releases).

### Windows

Extract the Windows release package and run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -BinaryPath .\nprobe.exe
```

Restart PowerShell and verify:

```powershell
nprobe --version
```

### Linux

Extract the Linux release package and run:

```bash
bash ./install.sh ./nprobe
```

Ensure `~/.local/bin` is included in your `PATH`.

### Build from source

```bash
git clone https://github.com/MajkiiWasTaken/NetProbe.git
cd NetProbe
cargo build --release
```

## Usage

| Command | Description |
|---|---|
| `nprobe dns example.com` | Resolve DNS addresses |
| `nprobe ping 192.168.1.1` | ICMP ping |
| `nprobe tcp 192.168.1.20 22` | Test TCP connectivity |
| `nprobe scan 192.168.1.20 --ports 22,80,443` | Scan TCP ports |
| `nprobe discover 192.168.1.0/24` | Discover network devices |
| `nprobe service ssh raspberry-pi` | Identify SSH service |
| `nprobe service https raspberry-pi` | Check HTTPS response |
| `nprobe service modbus 192.168.1.20` | Modbus TCP identification |
| `nprobe service mqtt 192.168.1.20` | MQTT handshake |
| `nprobe monitor raspberry-pi --service ssh` | Monitor TCP availability |
| `nprobe device list` | List device profiles |

Run `nprobe --help` for all commands and options.

## Device Profiles

Save devices for easier access:

```powershell
nprobe device add raspberry-pi --host 192.168.1.20
nprobe device info raspberry-pi
nprobe tcp raspberry-pi 22
```

Configuration is stored in `%APPDATA%\NetProbe\config.toml` on Windows or `~/.config/netprobe/config.toml` on Linux.

Example:

```toml
[devices.raspberry-pi]
host = "192.168.1.20"
description = "Raspberry Pi"

[devices.raspberry-pi.services.ssh]
port = 22

[devices.raspberry-pi.services.http]
port = 80
```

## Monitoring

Monitor a device and export results to JSON:

```powershell
nprobe monitor raspberry-pi --service ssh --interval 2 --count 10 --json monitor.json
```

NetProbe records connection latency, availability, outages, and timestamps. Use `Ctrl+C` to stop continuous monitoring.

## Development

```bash
cargo fmt --check
cargo check
cargo test
cargo build --release
```

## Notes

- Network discovery detects TCP-responsive hosts, not every connected device.
- HTTPS verifies certificates by default; `--insecure` disables verification for trusted diagnostic environments.
- Modbus TCP identification may not be supported by every device.
- Monitoring measures TCP connection latency, not ICMP latency.
- Only scan networks you own or are authorized to administer.

## Author

**Michal Švrček**

## License

Distributed under the MIT License. See [LICENSE](LICENSE).