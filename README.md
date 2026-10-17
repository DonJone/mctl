# mctl

[![License: Non-Commercial Ethical](https://img.shields.io/badge/License-NC--ESL-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Windows%20%7C%20macOS-0078D6.svg)](#)
[![Language](https://img.shields.io/badge/Language-Rust%202021-DEA584.svg)](#)

A high-performance, zero-dependency command-line management tool for dedicated Minecraft servers.

`mctl` provides unified daemon control, native RCON protocol execution, Server List Ping (SLP) latency probing, and zero-lock hot backups across **Linux**, **Windows**, and **macOS**.

---

## Supported Operating Systems

* **Linux**: Native `systemd` service management, cgroup v2 memory tracking, `journalctl` log streaming, and POSIX exit code compliance.
* **Windows**: Native Windows Service and process supervisor, console log tailing, and automated background execution.
* **macOS**: Native `launchd` supervision and local process management for development and server hosting.

---

## Core Capabilities

* **Zero Runtime Dependencies**: Compiled into a single standalone static binary (< 2 MB) with instant startup (< 2ms) and minimal memory footprint.
* **Built-in Pure-Rust RCON Client**: Full implementation of the Minecraft Source RCON protocol supporting single-command execution, color code stripping, and an interactive command shell.
* **Built-in Server List Ping (SLP) Engine**: Queries server status directly over the Minecraft game port (TCP 25565), reporting live latency, MOTD, server protocol version, and online player counts even when RCON is unavailable.
* **Zero-Lock Hot Backup**: Flushes world data via RCON (`save-off`, `save-all flush`) and creates compressed archives (`.tar.gz`) on-the-fly while ignoring runtime lock files (`session.lock`). Includes automated rolling retention pruning.
* **Automatic Config Discovery**: Traverses the filesystem to locate `server.properties` and dynamically extracts ports, world directory, and RCON credentials.
* **Machine-Readable JSON Output**: All major commands support `--json` for seamless integration into Prometheus, web dashboards, Telegram/Discord bots, and CI/CD automation.

---

## Command Reference

```text
Usage: mctl [OPTIONS] [COMMAND]

Commands:
  status   Display service status, memory, CPU, port, and online players
  start    Start the Minecraft server daemon/service
  stop     Gracefully save world data and stop the server
  restart  Perform a clean graceful restart of the server
  reload   Reload server configuration and datapacks via RCON
  rcon     Send commands to server via RCON (drops to interactive shell if empty)
  ping     Probe Minecraft server status using native Server List Ping (SLP) protocol
  log      View or stream server logs (tail -f follow mode supported)
  backup   Create a zero-lock compressed hot backup archive of the active world
  license  Display software license terms
  help     Print this message or the help of the given subcommand(s)

Options:
      --json               Output results in structured JSON format
  -d, --dir <PATH>         Working directory of the Minecraft server
  -s, --service <NAME>     System service name (default: minecraft)
  -H, --host <HOST>        RCON server host (default: 127.0.0.1)
  -p, --port <PORT>        RCON server port (default: auto-detect from server.properties)
  -P, --password <PASS>    RCON server password (default: auto-detect from server.properties)
  -h, --help               Print help
  -V, --version            Print version
```

---

## Usage Examples

### 1. Inspect Server Status
```bash
# Human-readable formatted terminal output
mctl status

# Machine-readable JSON output for monitoring pipelines
mctl --json status
```

### 2. Execute RCON Commands
```bash
# Single command
mctl rcon list
mctl rcon "say [Server Notice] Maintenance in 5 minutes"

# Interactive RCON console
mctl rcon
```

### 3. Probe Server Health (SLP)
```bash
# Probe local server
mctl ping

# Probe remote server
mctl ping mc.example.com:25565
```

### 4. Zero-Lock Hot Backup
```bash
# Backup world to default backup directory with 7-day retention
mctl backup

# Backup world to custom directory with 14-day retention
mctl backup -o /mnt/backups -r 14
```

### 5. Inspect and Stream Logs
```bash
# View last 50 log lines
mctl log -n 50

# Live follow mode (tail -f)
mctl log -f
```

---

## Installation & Building

### Prerequisites
* Rust toolchain (1.75+ or newer, with `cargo`)

### Build from Source
```bash
git clone https://github.com/DonJone/mctl.git
cd mctl
cargo build --release

# Binary is available at target/release/mctl
# Install to system PATH:
sudo cp target/release/mctl /usr/local/bin/mctl
```

---

## License

Licensed under the **Non-Commercial Ethical Source License (NC-ESL)**.  
See [LICENSE](LICENSE) for details.  
Copyright (c) 2026 DonJone (https://github.com/DonJone).
