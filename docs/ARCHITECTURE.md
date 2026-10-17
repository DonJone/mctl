# Production Architecture: Dedicated Minecraft Server Deployment

This document outlines the architecture for deploying dedicated Minecraft servers with automated lifecycle management, high-performance monitoring, and reverse-proxy network tunneling across Linux, Windows, and macOS.

---

## 1. High-Level Architecture Topology

```text
+-----------------------------------------------------------------------------------+
|                                  PUBLIC INTERNET                                  |
|                                                                                   |
|    Remote Players (Java Client)             Remote Friends (Simple Voice Chat)    |
|                 |                                           |                     |
|                 v                                           v                     |
|         TCP 25565 / DNS                             UDP 24454 / DNS               |
+-----------------------------------------------------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                             CLOUD EDGE RELAY (frps)                               |
|                                                                                   |
|  - Cloud Linux Instance (Public Relay)                                            |
|  - Reverse Proxy via FRP Server (frps)                                            |
|  - Exposes Game Port (TCP 25565), Voice Port (UDP 24454), SSH (TCP 60022)         |
+-----------------------------------------------------------------------------------+
                                          |
                                          | Encrypted Tunnel
                                          v
+-----------------------------------------------------------------------------------+
|                        LOCAL HOST: LINUX / WINDOWS / MACOS                        |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  | Dedicated Headless Server (Daemon)                                          |  |
|  | - Server Engine: Fabric / Forge / NeoForge / Paper / Purpur / Vanilla       |  |
|  | - Runtime: OpenJDK 21 LTS (G1GC High-Throughput Tuning)                     |  |
|  | - Service Manager: systemd (Linux), Windows Service, launchd (macOS)        |  |
|  | - Auto-restart on failure, scheduled timer backups                          |  |
|  +-----------------------------------------------------------------------------+  |
|                                         ^                                         |
|                                         | Loopback RCON:25575 / SLP:25565         |
|  +-----------------------------------------------------------------------------+  |
|  | Native Management CLI: mctl (Rust)                                          |  |
|  | - Unified status, start, stop, restart, reload, logs, backups, and RCON    |  |
|  | - Zero-lock hot backup engine (automatic chunk flush, session.lock bypass)  |  |
|  | - Machine-readable JSON metrics for monitoring and bots                     |  |
|  +-----------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------+
```

---

## 2. Key Architecture Pillars

### 2.1 Service Supervision & Process Isolation
* **Linux**: Systemd integration with `systemctl` lifecycle commands, cgroup v2 resource limits, and `journalctl` log persistence.
* **Windows**: Dedicated service wrapper (`winsw` / native service) or Task Scheduler supervisor.
* **macOS**: `launchd` service definitions.

### 2.2 Zero-Lock Hot Backups
`mctl backup` ensures 100% data integrity without taking the server offline:
1. RCON sends `save-off` and `save-all flush` to ensure all modified chunks are committed to disk.
2. The world directory is compressed into `.tar.gz` while safely skipping the runtime exclusive `session.lock`.
3. An RAII guard immediately restores `save-on` upon completion or error.
4. Retention policy automatically purges backups older than the retention threshold.

### 2.3 Health Probing & Metrics
* **SLP Probing**: Checks protocol level responsiveness, latency in milliseconds, MOTD, and online players without requiring administrative RCON access.
* **Structured JSON**: Output can be piped into monitoring systems and alerts.
