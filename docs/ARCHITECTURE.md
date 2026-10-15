# Production Architecture: Headless Dedicated Minecraft Server on Windows Desktop

This document outlines the architecture for transforming a high-performance Windows desktop into a 24/7 headless Minecraft dedicated server, while keeping the machine fully usable as a regular gaming PC with zero impact on the local player.

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
|  - Cloud Linux Instance (Public IP: 1.2.3.4)                                      |
|  - Reverse Proxy via FRP Server (frps)                                            |
|  - Exposes Game Port (TCP 25565), Voice Port (UDP 24454), SSH (TCP 60022)         |
+-----------------------------------------------------------------------------------+
                                          |
                                          | FRP Encrypted Tunnel
                                          v
+-----------------------------------------------------------------------------------+
|                        LOCAL HOST: WINDOWS 11 DESKTOP                             |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  | Hardware & Power Management                                                 |  |
|  | - Zen 4 / Modern Multi-core CPU + DDR5 RAM + NVMe SSD                       |  |
|  | - OLED Display Protection: Auto display-off (5 min), System Sleep DISABLED  |  |
|  +-----------------------------------------------------------------------------+  |
|                                                                                   |
|  +-----------------------------+         +-------------------------------------+  |
|  | Local Player Client (GUI)   |         | Dedicated Headless Server (Daemon)  |  |
|  | - Path: D:\Games\Minecraft\ |         | - Path: E:\MinecraftServer\         |  |
|  | - Launcher: PCL2 / Official |         | - Fabric Loader + Java 21 LTS       |  |
|  | - Connects to: 127.0.0.1    |         | - Managed by Windows Task Scheduler |  |
|  | - Network Latency: 0ms      |         | - Auto-restart on system boot       |  |
|  +-----------------------------+         +-------------------------------------+  |
|                                                             ^                     |
|                                                             | Loopback RCON:25575 |
|  +----------------------------------------------------------+------------------+  |
|  | Native Management CLI: mctl                                                 |  |
|  | - Status, start, stop, restart, reload, logs, backups, and RCON              |  |
|  +-----------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------+
```

---

## 2. Key Architecture Pillars

### 2.1 Complete Physical Isolation
The dedicated server (`E:\MinecraftServer\`) and the client game launcher (`D:\Games\Minecraft\`) operate in separate disk directories:
* **Zero Contention**: The host gamer can open/close the client at any time without terminating the server.
* **Shared Mods**: Server-side mods (e.g. Cobblemon, Fabric API, Spark) run headless in background; graphical mods (Sodium, Iris shaders) remain strictly in client.

### 2.2 Daemonized Service via Task Scheduler
Instead of running in an interactive terminal window that could accidentally be closed:
* Registered as a Windows Scheduled Task (`MinecraftServer`) running under `SYSTEM` or dedicated administrative account.
* Configured with `At startup` trigger for automatic boot without requiring user logon.
* Managed natively via `mctl start`, `mctl stop`, `mctl restart`.

### 2.3 OLED Burn-In Prevention with 24/7 Uptime
Running a server on a PC connected to an OLED monitor requires strict power management:
1. **Disable Sleep/Hibernation**:
   ```cmd
   powercfg /change standby-timeout-ac 0
   powercfg /hibernate off
   ```
2. **Enable Video Signal Cutoff (5 Minutes)**:
   ```cmd
   powercfg /change monitor-timeout-ac 5
   ```
   After 5 minutes of keyboard/mouse inactivity, the GPU shuts off HDMI/DP display output. The OLED panel completely powers down all pixels (0W, zero burn-in risk), while CPU, RAM, network, and Minecraft continue at full speed.

### 2.4 Live Zero-Lock World Hot Backup
Minecraft locks `world\session.lock` during runtime. Traditional file copiers fail with file locking errors.
`mctl backup` implements:
1. RCON `save-all` flush to sync dirty chunks from RAM to disk.
2. File streaming with `FileShare.ReadWrite`.
3. Explicit bypass of runtime lock `session.lock`.
4. Automated retention policy pruning archives older than 7 days.
