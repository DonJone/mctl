# mctl

[![License: Non-Commercial Ethical](https://img.shields.io/badge/License-NC--ESL-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6.svg)](#)
[![Runtime](https://img.shields.io/badge/.NET%20Framework-4.0%2B%20%28Built--in%29-512BD4.svg)](#)
[![Language](https://img.shields.io/badge/Language-C%23%205-239120.svg)](#)

> Lightweight, zero-dependency, native Windows CLI controller and production headless daemon architecture for dedicated Minecraft servers.

[English](#features) | [中文说明](#中文说明)

---

## Features

* **Zero Runtime Dependencies**: Written in pure C# (compatible with .NET 4.0+ / C# 5). Compiles natively using Windows built-in `csc.exe` without installing Visual Studio, SDKs, or external package managers.
* **Unix-Compliant CLI**: Clean command structure (`status`, `start`, `stop`, `restart`, `reload`, `rcon`, `log`, `backup`, `license`) following POSIX exit code conventions (e.g. exit code `3` for inactive service).
* **Automatic Config Discovery**: Dynamically resolves server working directories and extracts RCON port, server port, and RCON credentials directly from `server.properties`—no hardcoded passwords in binaries.
* **Live Zero-Lock Hot Backup**: Archives multi-gigabyte active worlds safely on-the-fly without server downtime by using streaming `FileShare.ReadWrite` and automatically skipping Minecraft's exclusive runtime `session.lock`. Includes automated 7-day rolling retention.
* **Real-Time Log Streaming**: Native `log -f` (tail -f) implementation for inspecting live server events directly over SSH or local consoles.
* **Bilingual Localization**: Automatically detects operating system language with seamless Chinese (`zh`) and English (`en`) support, plus manual `-l <zh|en>` flag overrides.
* **OLED-Safe Headless Architecture**: Designed for running 24/7 dedicated game servers on personal gaming desktop rigs while fully protecting OLED displays against burn-in.

---

## Quick Start & Installation

### Compile in 1 Second (No SDK Required)

On any Windows 10 or 11 system with administrative PowerShell or CMD, run:

```cmd
C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe /target:exe /optimize+ /r:System.IO.Compression.FileSystem.dll /r:System.IO.Compression.dll /out:C:\Windows\System32\mctl.exe src\mctl.cs
```

Because `mctl.exe` is placed into `C:\Windows\System32\`, it is immediately available in your global `PATH`.

---

## Command Reference

```text
Usage: mctl <command> [options]

Commands:
  status              Display service status, memory, CPU, port, and online players
  start               Start the server daemon
  stop                Gracefully save data and stop the server
  restart             Perform a clean restart
  reload              Reload server configuration and datapacks
  rcon <cmd...>       Execute a console command via RCON
  log [-n N] [-f]     View server logs (supports tail -f follow mode)
  backup              Create a zero-lock compressed backup archive of the world
  license             Display software license terms

Options:
  -l, --lang <zh|en>  Set display language (zh / en)
  -v, --version       Display version information
  -h, --help          Show help message
```

### Examples

```cmd
:: Check live status and metrics
mctl status

:: Send in-game broadcast
mctl rcon say Server restarting in 5 minutes!

:: Inspect recent 50 lines of logs
mctl log -n 50

:: Follow live logs in real time
mctl log -f

:: Create instant hot backup
mctl backup
```

---

## Production Headless Architecture

For detailed blueprints on setting up a 24/7 headless dedicated server on a Windows desktop—including FRP reverse proxying, Windows Task Scheduler daemonization, and OLED burn-in prevention—please consult:

* [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
* [examples/frpc.toml.example](examples/frpc.toml.example)
* [examples/run-mc.bat.example](examples/run-mc.bat.example)
* [examples/server.properties.example](examples/server.properties.example)

---

<a name="中文说明"></a>
## 中文说明

`mctl` 是专为 Windows 桌面级 Minecraft 专用服务端设计的轻量级原生命令行控制器。

### 核心特性

1. **零外部依赖编译**：采用标准 C# 编写，直接利用 Windows 系统自带的 .NET Framework 编译器（`csc.exe`）一秒完成编译，无需安装任何额外开发工具。
2. **自动读取配置与免密交互**：自动探测服务端目录，动态解析 `server.properties` 中的 RCON 口令与端口，源码和二进制中不留存任何明文凭证。
3. **不停服热备份**：独创防文件锁流式压缩，自动绕过 `session.lock` 运行时锁，在玩家正常联机时安全生成完整世界快照，并自动轮转清理 7 天前旧档案。
4. **实时日志追踪**：内置类似于 Linux `tail -f` 的动态日志流模式（`mctl log -f`）。
5. **OLED 友好型常驻架构**：配合 Windows 电源策略实现“闭屏不关机”，彻底保护高刷 OLED 屏幕避免烧屏，同时保障服务端 24 小时满血稳定运行。

---

## License

This project is licensed under the **Non-Commercial Ethical Source License (NC-ESL)**.

* Free for personal, non-commercial, community, and educational use.
* Commercial hosting providers, cloud vendors, and AI dataset scrapers are strictly prohibited without prior written permission from the author.

Copyright (c) 2026 [DonJone](https://github.com/DonJone).
