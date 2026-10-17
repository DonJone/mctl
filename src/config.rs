use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub server_dir: PathBuf,
    pub service_name: String,
    pub server_port: u16,
    pub rcon_host: String,
    pub rcon_port: u16,
    pub rcon_password: Option<String>,
    pub level_name: String,
    #[allow(dead_code)]
    pub motd: String,
}

impl ServerConfig {
    pub fn resolve(
        cli_dir: Option<PathBuf>,
        cli_service: Option<String>,
        cli_host: Option<String>,
        cli_port: Option<u16>,
        cli_pass: Option<String>,
    ) -> Result<Self, String> {
        let server_dir = find_server_dir(cli_dir)?;
        let service_name = cli_service
            .or_else(|| env::var("MCTL_SERVICE").ok())
            .unwrap_or_else(|| "minecraft".to_string());

        let properties = read_server_properties(&server_dir.join("server.properties"));

        let server_port = properties
            .get("server-port")
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(25565);

        let rcon_host = cli_host
            .or_else(|| env::var("MCTL_RCON_HOST").ok())
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let rcon_port = cli_port
            .or_else(|| {
                env::var("MCTL_RCON_PORT")
                    .ok()
                    .and_then(|v| v.parse::<u16>().ok())
            })
            .or_else(|| {
                properties
                    .get("rcon.port")
                    .and_then(|v| v.parse::<u16>().ok())
            })
            .unwrap_or(25575);

        let rcon_password = cli_pass
            .or_else(|| env::var("MCTL_RCON_PASS").ok())
            .or_else(|| properties.get("rcon.password").cloned());

        let level_name = properties
            .get("level-name")
            .cloned()
            .unwrap_or_else(|| "world".to_string());

        let motd = properties
            .get("motd")
            .cloned()
            .unwrap_or_else(|| "A Minecraft Server".to_string());

        Ok(Self {
            server_dir,
            service_name,
            server_port,
            rcon_host,
            rcon_port,
            rcon_password,
            level_name,
            motd,
        })
    }
}

fn find_server_dir(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(dir) = explicit {
        if dir.exists() {
            return Ok(fs::canonicalize(dir).map_err(|e| e.to_string())?);
        }
        return Err(format!("Specified server directory does not exist: {:?}", dir));
    }

    if let Ok(env_dir) = env::var("MCTL_DIR") {
        let p = PathBuf::from(env_dir);
        if p.exists() {
            return Ok(fs::canonicalize(p).map_err(|e| e.to_string())?);
        }
    }

    // Check current working directory
    if let Ok(cwd) = env::current_dir() {
        if is_mc_dir(&cwd) {
            return Ok(cwd);
        }
        // Check parents
        let mut curr = cwd.clone();
        for _ in 0..3 {
            if let Some(parent) = curr.parent() {
                if is_mc_dir(parent) {
                    return Ok(parent.to_path_buf());
                }
                curr = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    // Check common standard locations
    let candidates = [
        "/home/gky/minecraft/server",
        "/home/don/minecraft/server",
        "/srv/minecraft",
        "/opt/minecraft",
        "E:\\MinecraftServer",
        "C:\\MinecraftServer",
    ];

    for candidate in candidates {
        let p = PathBuf::from(candidate);
        if p.exists() && is_mc_dir(&p) {
            return Ok(p);
        }
    }

    // Fallback to current directory
    env::current_dir().map_err(|e| format!("Failed to determine working directory: {}", e))
}

fn is_mc_dir(path: &Path) -> bool {
    path.join("server.properties").exists()
        || path.join("fabric-server-launch.jar").exists()
        || path.join("server.jar").exists()
}

fn read_server_properties(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}
