# NetProbe

**NetProbe** is a lightweight, cross-platform network diagnostics CLI written in Rust. It provides DNS resolution, ICMP ping, TCP connectivity checks, local-network discovery, service-level diagnostics, availability monitoring, and reusable device profiles.

Built for developers and administrators troubleshooting Linux devices, embedded gateways, lab networks, and everyday TCP services.

**Current version:** `v0.6.0`  
**Executable:** `nprobe`  
**Platforms:** Windows and Linux

## Features

- **DNS lookup** — resolve hostnames to IP addresses.
- **Ping** — invoke the system ICMP ping utility.
- **TCP connectivity** — test whether a host accepts connections on a specific port.
- **TCP port scan** — test selected ports or ranges on one host.
- **Network discovery** — concurrently check selected TCP ports across a local IPv4 subnet (up to 256 addresses per scan).
- **Service diagnostics** — identify SSH banners, retrieve HTTP/HTTPS responses, send a read-only Modbus TCP device-identification request, and attempt a basic MQTT handshake.
- **Monitoring** — repeat TCP checks, measure connection latency, detect outages and recoveries, and export a JSON report.
- **Device profiles** — save device addresses and named services in a TOML configuration file.
- **Colored terminal output** — clear success, warning, informational, and error messages.

> Network discovery finds **TCP-responsive hosts on the ports you choose**. It does not guarantee detection of every powered-on device.

## Installation

Download a package from [GitHub Releases](https://github.com/MajkiiWasTaken/NetProbe/releases), or build from source.

### Windows

Extract `NetProbe-windows-x86_64.zip`, open PowerShell in the extracted directory, and run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -BinaryPath .\nprobe.exe
```

The executable is installed to `%LOCALAPPDATA%\Programs\NetProbe`. Open a new terminal to refresh `PATH`:

```powershell
nprobe --version
nprobe --help
```

### Linux

Extract `NetProbe-linux-x86_64.tar.gz` and run:

```bash
bash ./install.sh ./nprobe
```

The executable is installed to `~/.local/bin/nprobe`. Ensure `~/.local/bin` is on your `PATH`.

### Build from source

Requires a current stable Rust toolchain and Cargo.

```bash
git clone https://github.com/MajkiiWasTaken/NetProbe.git
cd NetProbe
cargo build --release
```

On Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

On Linux:

```bash
bash scripts/install.sh
```

The uninstall scripts remove the installed executable and PATH entry (where applicable), but preserve saved device profiles.

## Command reference

| Command | Description |
| --- | --- |
| `nprobe dns example.com` | Resolve DNS records to IP addresses |
| `nprobe ping 192.168.1.1` | Run system ICMP ping |
| `nprobe tcp 192.168.1.20 22` | Test one TCP port |
| `nprobe scan 192.168.1.20` | Scan commonly used TCP ports |
| `nprobe scan 192.168.1.20 --ports 22,80,443,502` | Scan specific ports |
| `nprobe scan 192.168.1.20 --ports 8000-8010` | Scan a port range |
| `nprobe discover 192.168.1.0/24` | Find hosts responsive on selected TCP ports |
| `nprobe service ssh 192.168.1.20` | Read the SSH identification banner |
| `nprobe service http 192.168.1.20` | Request an HTTP response |
| `nprobe service https 192.168.1.20` | Request HTTPS with certificate verification |
| `nprobe service modbus 192.168.1.20` | Try Modbus TCP Read Device Identification |
| `nprobe service mqtt 192.168.1.20` | Try a basic MQTT connection handshake |
| `nprobe monitor 192.168.1.20 --port 22` | Monitor TCP availability |
| `nprobe device add gateway --host 192.168.1.20` | Save a device profile |
| `nprobe device list` | List configured devices |
| `nprobe device info gateway` | Show profile settings |
| `nprobe device remove gateway --yes` | Remove a saved profile |

Run `nprobe <command> --help` for the complete, up-to-date argument list.

## Network discovery

Scan a private IPv4 subnet with the default ports:

```powershell
nprobe discover 192.168.1.0/24
```

Choose ports and concurrency:

```powershell
nprobe discover 192.168.1.0/24 --ports 22,80,443,502,1883,8080 --threads 32 --timeout-ms 250
```

- Discovery uses bounded concurrent TCP probes, not an ARP inventory or a comprehensive network scanner.
- A host is reported when at least one selected port accepts a TCP connection.
- The initial discovery implementation limits scans to **256 IPv4 addresses**, local/private ranges, and at most **64 workers**.
- Closed, filtered, or untested ports can cause running devices to be absent from results.

Only scan networks you own or are authorized to administer.

## Service diagnostics

### SSH

```powershell
nprobe service ssh gateway
```

Reads the server's SSH identification line (for example, `SSH-2.0-dropbear_2020.81`). It does **not** authenticate or open a shell.

### HTTP and HTTPS

```powershell
nprobe service http 192.168.1.20
nprobe service https gateway
```

HTTP checks report the HTTP status code and available `Server` header. HTTPS verifies the server certificate by default. For a trusted lab device with an untrusted or self-signed certificate, an explicit diagnostic bypass is available:

```powershell
nprobe service https gateway --insecure
```

`--insecure` disables TLS certificate verification. Use it only when you understand and accept the security risk; prefer a trusted CA and correctly configured certificate.

An HTTP redirect (for example, status `303`) is a valid HTTP response, not necessarily a service failure.

### Modbus TCP

```powershell
nprobe service modbus 192.168.1.20 --unit-id 1 --timeout-ms 5000
```

The diagnostic attempts **Read Device Identification** (function 43/14). Not every Modbus TCP server implements it. A reachable TCP port `502` does **not** establish that the request is supported; a failed identification probe does **not** by itself prove the Modbus service is broken. Only probe industrial devices you are authorized to test.

### MQTT

```powershell
nprobe service mqtt 192.168.1.20
```

Attempts an MQTT 3.1.1 CONNECT handshake without credentials or publishing messages. Authentication requirements can prevent a successful application-level connection.

## Device profiles

Save frequently used devices:

```powershell
nprobe device add gateway --host 192.168.1.20 --description "Industrial gateway"
nprobe device list
nprobe device info gateway
```

Configuration locations:

- **Windows:** `%APPDATA%\NetProbe\config.toml`
- **Linux:** `~/.config/netprobe/config.toml` (or `$XDG_CONFIG_HOME/netprobe/config.toml`)

Example TOML:

```toml
[devices.gateway]
host = "192.168.1.20"
description = "Industrial gateway"

[devices.gateway.services.ssh]
port = 22
insecure = false

[devices.gateway.services.https]
port = 443
insecure = false

[devices.gateway.services.modbus]
port = 502
insecure = false
```

The device name can be used instead of a hostname where profile resolution is supported:

```powershell
nprobe tcp gateway 22
nprobe service ssh gateway
nprobe service https gateway
nprobe monitor gateway --service ssh --count 5
```

Explicit `--port` arguments override configured service ports. The `insecure` setting applies to HTTPS service diagnostics, **not** TCP-only monitoring. Avoid enabling it by default.

## Monitoring and JSON history

Monitor a device's TCP availability at a fixed interval:

```powershell
nprobe monitor gateway --service ssh --interval 5 --timeout-ms 1000
```

Stop with `Ctrl+C`, or provide `--count` for a finite run:

```powershell
nprobe monitor gateway --service ssh --interval 2 --count 10 --json gateway-monitor.json
```

The console summarizes successful and failed checks, availability percentage, detected outage transitions, and connection latency. JSON reports include timestamps, per-check results, errors, and aggregate statistics.

**Latency measures the TCP connection establishment time**, not ICMP round-trip time or an end-to-end application request.

> JSON is written on normal completion or a handled `Ctrl+C`. A forced termination, process crash, or power failure may leave no report. Monitoring data can contain internal network addresses; review it before sharing.

## Development

```powershell
cargo fmt --check
cargo check
cargo test
cargo build --release
```

To verify an installation:

```powershell
nprobe --version
nprobe device list
nprobe tcp gateway 22
nprobe service ssh gateway
nprobe monitor gateway --service ssh --count 5
```

## Releases

Release builds use GitHub Actions to package:

- `NetProbe-windows-x86_64.zip`
- `NetProbe-linux-x86_64.tar.gz`

To publish an official version after successful tests:

```powershell
git add .
git commit -m "release: NetProbe v0.6.0"
git push origin main
git tag v0.6.0
git push origin v0.6.0
```

Create the tag only if it does not already exist. Publishing requires the repository's release workflow and appropriate GitHub permissions.

## Limitations and security

- NetProbe is intended for diagnostics, not vulnerability assessment or exhaustive service discovery.
- TCP reachability is not equivalent to application health.
- Some protocol checks create short-lived connections or send protocol requests; use care with production and industrial equipment.
- Local device profiles are stored in plaintext. Do not store credentials or private keys in them.
- TLS verification should stay enabled except for explicitly trusted diagnostic environments.
- Monitoring availability is **sample-based** and can miss outages occurring between checks.

## Author

**Michal Švrček**  

## License

MIT License. See [LICENSE](LICENSE).
