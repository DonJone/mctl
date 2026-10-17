mod backup;
mod cli;
mod config;
mod rcon;
mod service;
mod slp;

use clap::Parser;
use cli::{Cli, Commands};
use config::ServerConfig;
use rcon::RconClient;
use serde_json::json;
use service::ServiceManager;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let cfg_result = ServerConfig::resolve(
        cli.server_dir.clone(),
        cli.service_name.clone(),
        cli.rcon_host.clone(),
        cli.rcon_port,
        cli.rcon_password.clone(),
    );

    let cfg = match cfg_result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration Error: {}", e);
            return ExitCode::from(1);
        }
    };

    let service = ServiceManager::new(cfg.service_name.clone());

    let cmd = cli.command.unwrap_or(Commands::Status);

    match cmd {
        Commands::Status => handle_status(&cfg, &service, cli.json),
        Commands::Start => handle_start(&service, cli.json),
        Commands::Stop => handle_stop(&cfg, &service, cli.json),
        Commands::Restart => handle_restart(&cfg, &service, cli.json),
        Commands::Reload => handle_reload(&cfg, cli.json),
        Commands::Rcon { command } => handle_rcon(&cfg, command),
        Commands::Ping { address } => handle_ping(&cfg, address, cli.json),
        Commands::Log { lines, follow } => handle_log(&cfg, &service, lines, follow),
        Commands::Backup {
            output_dir,
            retention_days,
        } => handle_backup(&cfg, output_dir, retention_days, cli.json),
        Commands::License => {
            print_license();
            ExitCode::SUCCESS
        }
    }
}

fn handle_status(cfg: &ServerConfig, service: &ServiceManager, json_mode: bool) -> ExitCode {
    let s_info = service.status();

    // Query SLP ping on port
    let slp_info = slp::ping(&cfg.rcon_host, cfg.server_port, Duration::from_millis(1500)).ok();

    // Query RCON if password is present and service active
    let mut rcon_players: Option<String> = None;
    if s_info.is_active {
        if let Some(ref pass) = cfg.rcon_password {
            if let Ok(mut client) = RconClient::connect(&cfg.rcon_host, cfg.rcon_port, pass, Duration::from_millis(1500)) {
                if let Ok(resp) = client.exec("list") {
                    rcon_players = Some(resp.trim().to_string());
                }
            }
        }
    }

    if json_mode {
        let out = json!({
            "service_name": cfg.service_name,
            "server_dir": cfg.server_dir.to_string_lossy(),
            "active": s_info.is_active,
            "pid": s_info.pid,
            "memory_mb": s_info.memory_mb,
            "uptime": s_info.uptime_str,
            "service_type": s_info.service_type,
            "server_port": cfg.server_port,
            "rcon_port": cfg.rcon_port,
            "slp": slp_info,
            "rcon_players": rcon_players,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("Service Name:   {} ({})", cfg.service_name, s_info.service_type);
        println!("Working Dir:    {}", cfg.server_dir.display());

        if s_info.is_active {
            let uptime = s_info.uptime_str.unwrap_or_else(|| "Running".to_string());
            let pid = s_info.pid.map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string());
            println!("Status:         active (running since {})", uptime);
            println!("Main PID:       {}", pid);

            if let Some(mb) = s_info.memory_mb {
                let gb = (mb as f64) / 1024.0;
                println!("Memory:         {} MB ({:.2} GB)", mb, gb);
            }
        } else {
            println!("Status:         inactive (stopped)");
        }

        if let Some(ref slp) = slp_info {
            println!("Port {}:      listening ({}ms latency)", cfg.server_port, slp.latency_ms);
            println!("Version:        {} (protocol {})", slp.version_name, slp.protocol);
            println!("MOTD:           {}", slp.motd);
            println!("Online Players: {}/{}", slp.players_online, slp.players_max);
            if !slp.player_samples.is_empty() {
                println!("Players Sample: {}", slp.player_samples.join(", "));
            }
        } else {
            println!("Port {}:      not reachable", cfg.server_port);
        }

        if let Some(ref list_str) = rcon_players {
            println!("RCON Players:   {}", list_str);
        }
    }

    if s_info.is_active {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3) // POSIX standard for inactive service
    }
}

fn handle_start(service: &ServiceManager, json_mode: bool) -> ExitCode {
    println!("Starting server service...");
    match service.start() {
        Ok(_) => {
            if json_mode {
                println!("{}", json!({ "status": "success", "action": "start" }));
            } else {
                println!("Service started successfully.");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error starting service: {}", e);
            ExitCode::from(1)
        }
    }
}

fn handle_stop(cfg: &ServerConfig, service: &ServiceManager, json_mode: bool) -> ExitCode {
    // Attempt graceful RCON save-all before stopping
    if let Some(ref pass) = cfg.rcon_password {
        if let Ok(mut client) = RconClient::connect(&cfg.rcon_host, cfg.rcon_port, pass, Duration::from_secs(2)) {
            println!("Saving world data via RCON...");
            client.exec("save-all").ok();
            std::thread::sleep(Duration::from_millis(1000));
        }
    }

    println!("Stopping server service...");
    match service.stop() {
        Ok(_) => {
            if json_mode {
                println!("{}", json!({ "status": "success", "action": "stop" }));
            } else {
                println!("Service stopped successfully.");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error stopping service: {}", e);
            ExitCode::from(1)
        }
    }
}

fn handle_restart(cfg: &ServerConfig, service: &ServiceManager, json_mode: bool) -> ExitCode {
    if let Some(ref pass) = cfg.rcon_password {
        if let Ok(mut client) = RconClient::connect(&cfg.rcon_host, cfg.rcon_port, pass, Duration::from_secs(2)) {
            println!("Saving world data before restart...");
            client.exec("save-all").ok();
            std::thread::sleep(Duration::from_millis(1000));
        }
    }

    println!("Restarting server service...");
    match service.restart() {
        Ok(_) => {
            if json_mode {
                println!("{}", json!({ "status": "success", "action": "restart" }));
            } else {
                println!("Service restarted successfully.");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error restarting service: {}", e);
            ExitCode::from(1)
        }
    }
}

fn handle_reload(cfg: &ServerConfig, json_mode: bool) -> ExitCode {
    let pass = match cfg.rcon_password {
        Some(ref p) => p.as_str(),
        None => {
            eprintln!("RCON password is not configured in server.properties or environment.");
            return ExitCode::from(1);
        }
    };

    match RconClient::connect(&cfg.rcon_host, cfg.rcon_port, pass, Duration::from_secs(3)) {
        Ok(mut client) => match client.exec("reload") {
            Ok(resp) => {
                if json_mode {
                    println!("{}", json!({ "status": "success", "response": resp.trim() }));
                } else {
                    println!("{}", resp.trim());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("RCON command error: {}", e);
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("RCON connection failed: {}", e);
            ExitCode::from(1)
        }
    }
}

fn handle_rcon(cfg: &ServerConfig, command: Vec<String>) -> ExitCode {
    let pass = match cfg.rcon_password {
        Some(ref p) => p.as_str(),
        None => {
            eprintln!("RCON password is not configured in server.properties or environment.");
            return ExitCode::from(1);
        }
    };

    let mut client = match RconClient::connect(&cfg.rcon_host, cfg.rcon_port, pass, Duration::from_secs(5)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to RCON: {}", e);
            return ExitCode::from(1);
        }
    };

    if command.is_empty() {
        // Drop into interactive shell
        match client.interactive_shell() {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Interactive shell error: {}", e);
                ExitCode::from(1)
            }
        }
    } else {
        let cmd_str = command.join(" ");
        match client.exec(&cmd_str) {
            Ok(resp) => {
                if !resp.is_empty() {
                    println!("{}", resp.trim());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("RCON command failed: {}", e);
                ExitCode::from(1)
            }
        }
    }
}

fn handle_ping(cfg: &ServerConfig, address: Option<String>, json_mode: bool) -> ExitCode {
    let (host, port) = if let Some(addr) = address {
        if let Some((h, p_str)) = addr.split_once(':') {
            let p = p_str.parse::<u16>().unwrap_or(cfg.server_port);
            (h.to_string(), p)
        } else {
            (addr, cfg.server_port)
        }
    } else {
        (cfg.rcon_host.clone(), cfg.server_port)
    };

    match slp::ping(&host, port, Duration::from_secs(3)) {
        Ok(status) => {
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
            } else {
                println!("Target:         {}:{}", status.host, status.port);
                println!("Status:         online ({}ms ping)", status.latency_ms);
                println!("Version:        {} (protocol {})", status.version_name, status.protocol);
                println!("MOTD:           {}", status.motd);
                println!("Players:        {}/{}", status.players_online, status.players_max);
                if !status.player_samples.is_empty() {
                    println!("Player List:    {}", status.player_samples.join(", "));
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Ping failed for {}:{}: {}", host, port, e);
            ExitCode::from(1)
        }
    }
}

fn handle_log(cfg: &ServerConfig, service: &ServiceManager, lines: usize, follow: bool) -> ExitCode {
    let log_file = cfg.server_dir.join("logs").join("latest.log");
    match service.view_logs(&log_file, lines, follow) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Log viewing error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn handle_backup(
    cfg: &ServerConfig,
    output_dir: Option<std::path::PathBuf>,
    retention_days: u32,
    json_mode: bool,
) -> ExitCode {
    let mut rcon_client = cfg
        .rcon_password
        .as_deref()
        .and_then(|pass| RconClient::connect(&cfg.rcon_host, cfg.rcon_port, pass, Duration::from_secs(2)).ok());

    match backup::create_hot_backup(
        &cfg.server_dir,
        &cfg.level_name,
        output_dir,
        retention_days,
        rcon_client.as_mut(),
    ) {
        Ok(res) => {
            if json_mode {
                println!(
                    "{}",
                    json!({
                        "status": "success",
                        "archive": res.archive_path.to_string_lossy(),
                        "size_mb": res.archive_size_mb,
                        "purged_old_archives": res.purged_count,
                    })
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Backup failed: {}", e);
            ExitCode::from(1)
        }
    }
}

fn print_license() {
    println!("Non-Commercial Ethical Source License (NC-ESL)");
    println!("Copyright (c) 2026 DonJone (https://github.com/DonJone)");
    println!();
    println!("Permission is hereby granted, free of charge, to any person obtaining a copy");
    println!("of this software, to use, copy, modify, and distribute for non-commercial purposes.");
    println!();
    println!("Commercial hosting and cloud exploitation without prior written consent is strictly prohibited.");
}
