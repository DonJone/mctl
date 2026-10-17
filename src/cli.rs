use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mctl",
    author = "DonJone <https://github.com/DonJone>",
    version = "2.0.0",
    about = "High-performance, cross-platform Minecraft dedicated server CLI management tool",
    long_about = "mctl is a zero-dependency, high-performance CLI controller for Minecraft dedicated servers.\nFully supporting Linux (systemd/cgroups), Windows, and macOS with built-in RCON, SLP ping, and zero-lock hot backup engines."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output results in structured JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Working directory of the Minecraft server
    #[arg(short = 'd', long = "dir", global = true, value_name = "PATH")]
    pub server_dir: Option<PathBuf>,

    /// System service name (e.g., minecraft)
    #[arg(short = 's', long = "service", global = true, value_name = "NAME")]
    pub service_name: Option<String>,

    /// RCON server host (default: 127.0.0.1)
    #[arg(short = 'H', long = "host", global = true, value_name = "HOST")]
    pub rcon_host: Option<String>,

    /// RCON server port (default: auto-detect from server.properties or 25575)
    #[arg(short = 'p', long = "port", global = true, value_name = "PORT")]
    pub rcon_port: Option<u16>,

    /// RCON server password (default: auto-detect from server.properties)
    #[arg(short = 'P', long = "password", global = true, value_name = "PASSWORD")]
    pub rcon_password: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Display service status, memory, CPU, port, and online players
    Status,

    /// Start the Minecraft server daemon/service
    Start,

    /// Gracefully save world data and stop the server
    Stop,

    /// Perform a clean graceful restart of the server
    Restart,

    /// Reload server configuration and datapacks via RCON
    Reload,

    /// Send commands to server via RCON (drops to interactive shell if empty)
    Rcon {
        /// RCON command string to execute (e.g. "list", "say hello")
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Probe Minecraft server status using native Server List Ping (SLP) protocol
    Ping {
        /// Target address in host:port format (default: 127.0.0.1:server_port)
        #[arg(value_name = "HOST[:PORT]")]
        address: Option<String>,
    },

    /// View or stream server logs (tail -f follow mode supported)
    Log {
        /// Number of recent log lines to display (default: 30)
        #[arg(short = 'n', long = "lines", default_value_t = 30)]
        lines: usize,

        /// Follow log output in real time
        #[arg(short = 'f', long = "follow")]
        follow: bool,
    },

    /// Create a zero-lock compressed hot backup archive of the active world
    Backup {
        /// Destination backup directory (default: <server_dir>/../backups or <server_dir>/backups)
        #[arg(short = 'o', long = "out")]
        output_dir: Option<PathBuf>,

        /// Retention period in days for rolling backup cleanup (default: 7)
        #[arg(short = 'r', long = "retention", default_value_t = 7)]
        retention_days: u32,
    },

    /// Display software license terms
    License,
}
