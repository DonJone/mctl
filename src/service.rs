use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceStatusInfo {
    pub is_active: bool,
    pub pid: Option<u32>,
    pub memory_mb: Option<u64>,
    pub uptime_str: Option<String>,
    pub service_type: String,
}

pub struct ServiceManager {
    service_name: String,
}

impl ServiceManager {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }

    pub fn status(&self) -> ServiceStatusInfo {
        #[cfg(target_os = "linux")]
        {
            if is_systemd_available() {
                if let Some(info) = get_systemd_status(&self.service_name) {
                    return info;
                }
            }
        }

        // Generic fallback check
        Self::fallback_status(&self.service_name)
    }

    pub fn start(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if is_systemd_available() {
                let status = Command::new("systemctl")
                    .args(["start", &self.service_name])
                    .status()
                    .map_err(|e| format!("Failed to execute systemctl start: {}", e))?;
                if status.success() {
                    return Ok(());
                }
                return Err(format!("systemctl start exited with status: {}", status));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let status = Command::new("sc")
                .args(["start", &self.service_name])
                .status()
                .map_err(|e| format!("Failed to execute sc start: {}", e))?;
            if status.success() {
                return Ok(());
            }
            return Err("Failed to start Windows service".to_string());
        }

        #[cfg(target_os = "macos")]
        {
            let status = Command::new("launchctl")
                .args(["start", &self.service_name])
                .status()
                .map_err(|e| format!("Failed to execute launchctl start: {}", e))?;
            if status.success() {
                return Ok(());
            }
            return Err("Failed to start launchd service".to_string());
        }

        #[allow(unreachable_code)]
        Err("No supported service manager found for this platform".to_string())
    }

    pub fn stop(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if is_systemd_available() {
                let status = Command::new("systemctl")
                    .args(["stop", &self.service_name])
                    .status()
                    .map_err(|e| format!("Failed to execute systemctl stop: {}", e))?;
                if status.success() {
                    return Ok(());
                }
                return Err(format!("systemctl stop exited with status: {}", status));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let status = Command::new("sc")
                .args(["stop", &self.service_name])
                .status()
                .map_err(|e| format!("Failed to execute sc stop: {}", e))?;
            if status.success() {
                return Ok(());
            }
            return Err("Failed to stop Windows service".to_string());
        }

        #[cfg(target_os = "macos")]
        {
            let status = Command::new("launchctl")
                .args(["stop", &self.service_name])
                .status()
                .map_err(|e| format!("Failed to execute launchctl stop: {}", e))?;
            if status.success() {
                return Ok(());
            }
            return Err("Failed to stop launchd service".to_string());
        }

        #[allow(unreachable_code)]
        Err("No supported service manager found for this platform".to_string())
    }

    pub fn restart(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if is_systemd_available() {
                let status = Command::new("systemctl")
                    .args(["restart", &self.service_name])
                    .status()
                    .map_err(|e| format!("Failed to execute systemctl restart: {}", e))?;
                if status.success() {
                    return Ok(());
                }
                return Err(format!("systemctl restart exited with status: {}", status));
            }
        }

        self.stop()?;
        sleep(Duration::from_secs(2));
        self.start()
    }

    pub fn view_logs(&self, log_file: &Path, lines: usize, follow: bool) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if is_systemd_available() {
                let mut cmd = Command::new("journalctl");
                cmd.args(["-u", &self.service_name, "-n", &lines.to_string()]);
                if follow {
                    cmd.arg("-f");
                }
                if let Ok(mut child) = cmd.spawn() {
                    child.wait().ok();
                    return Ok(());
                }
            }
        }

        // Native file tailing fallback for Windows, macOS, or non-systemd Linux
        if !log_file.exists() {
            return Err(format!("Log file not found: {:?}", log_file));
        }

        tail_file(log_file, lines, follow)
    }

    fn fallback_status(service_name: &str) -> ServiceStatusInfo {
        // Fallback: check if process is running via pgrep or tasklist
        #[cfg(unix)]
        {
            if let Ok(output) = Command::new("pgrep").args(["-f", "fabric-server|server.jar"]).output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if let Some(pid_str) = text.lines().next() {
                        if let Ok(pid) = pid_str.trim().parse::<u32>() {
                            return ServiceStatusInfo {
                                is_active: true,
                                pid: Some(pid),
                                memory_mb: None,
                                uptime_str: None,
                                service_type: "Process (Standalone)".to_string(),
                            };
                        }
                    }
                }
            }
        }

        ServiceStatusInfo {
            is_active: false,
            pid: None,
            memory_mb: None,
            uptime_str: None,
            service_type: format!("Service ({})", service_name),
        }
    }
}

#[cfg(target_os = "linux")]
fn is_systemd_available() -> bool {
    Path::new("/run/systemd/system").exists()
}

#[cfg(target_os = "linux")]
fn get_systemd_status(service_name: &str) -> Option<ServiceStatusInfo> {
    let output = Command::new("systemctl")
        .args([
            "show",
            service_name,
            "-p",
            "ActiveState",
            "-p",
            "MainPID",
            "-p",
            "ActiveEnterTimestamp",
            "-p",
            "MemoryCurrent",
        ])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut state = "";
    let mut pid: Option<u32> = None;
    let mut uptime: Option<String> = None;
    let mut mem_bytes: Option<u64> = None;

    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            match k {
                "ActiveState" => state = v,
                "MainPID" => {
                    if let Ok(p) = v.parse::<u32>() {
                        if p > 0 {
                            pid = Some(p);
                        }
                    }
                }
                "ActiveEnterTimestamp" => {
                    if !v.is_empty() {
                        uptime = Some(v.to_string());
                    }
                }
                "MemoryCurrent" => {
                    if let Ok(b) = v.parse::<u64>() {
                        if b > 0 && b != u64::MAX {
                            mem_bytes = Some(b);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let is_active = state == "active";
    let memory_mb = mem_bytes.map(|b| b / 1024 / 1024);

    Some(ServiceStatusInfo {
        is_active,
        pid,
        memory_mb,
        uptime_str: uptime,
        service_type: "systemd".to_string(),
    })
}

fn tail_file(path: &Path, lines_to_show: usize, follow: bool) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Cannot open log file: {}", e))?;
    let mut reader = BufReader::new(file);

    // Read all existing lines into a circular buffer or list
    let mut lines = Vec::new();
    let mut line = String::new();
    while reader.read_line(&mut line).map_err(|e| e.to_string())? > 0 {
        lines.push(line.clone());
        line.clear();
    }

    let start_idx = if lines.len() > lines_to_show {
        lines.len() - lines_to_show
    } else {
        0
    };

    for l in &lines[start_idx..] {
        print!("{}", l);
    }

    if !follow {
        return Ok(());
    }

    // Follow mode
    let mut cur_pos = reader.seek(SeekFrom::Current(0)).map_err(|e| e.to_string())?;
    loop {
        sleep(Duration::from_millis(250));
        let metadata = path.metadata().map_err(|e| e.to_string())?;
        let len = metadata.len();

        if len > cur_pos {
            let mut file = File::open(path).map_err(|e| e.to_string())?;
            file.seek(SeekFrom::Start(cur_pos)).map_err(|e| e.to_string())?;
            let mut r = BufReader::new(file);
            let mut l = String::new();
            while r.read_line(&mut l).map_err(|e| e.to_string())? > 0 {
                print!("{}", l);
                l.clear();
            }
            cur_pos = len;
        } else if len < cur_pos {
            // File truncated / rotated
            cur_pos = 0;
        }
    }
}
