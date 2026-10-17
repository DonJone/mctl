use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tar::Builder;

pub struct BackupResult {
    pub archive_path: PathBuf,
    pub archive_size_mb: u64,
    pub purged_count: usize,
}

pub struct SaveGuard<'a> {
    rcon: Option<&'a mut crate::rcon::RconClient>,
}

impl<'a> SaveGuard<'a> {
    pub fn new(mut rcon: Option<&'a mut crate::rcon::RconClient>) -> Self {
        if let Some(ref mut c) = rcon {
            println!("Flushing world data via RCON...");
            c.exec("save-off").ok();
            c.exec("save-all flush").ok();
            std::thread::sleep(Duration::from_millis(1500));
        }
        Self { rcon }
    }
}

impl<'a> Drop for SaveGuard<'a> {
    fn drop(&mut self) {
        if let Some(ref mut c) = self.rcon {
            c.exec("save-on").ok();
        }
    }
}

pub fn create_hot_backup(
    server_dir: &Path,
    world_name: &str,
    backup_dir: Option<PathBuf>,
    retention_days: u32,
    mut rcon: Option<&mut crate::rcon::RconClient>,
) -> Result<BackupResult, String> {
    let world_dir = server_dir.join(world_name);
    if !world_dir.exists() {
        return Err(format!("World directory does not exist: {:?}", world_dir));
    }

    let out_dir = backup_dir.unwrap_or_else(|| {
        let parent = server_dir.parent().unwrap_or(server_dir);
        let candidate = parent.join("backups");
        if candidate.exists() || parent != server_dir {
            candidate
        } else {
            server_dir.join("backups")
        }
    });

    fs::create_dir_all(&out_dir).map_err(|e| format!("Failed to create backup dir: {}", e))?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let archive_name = format!("{}_{}.tar.gz", world_name, timestamp);
    let archive_path = out_dir.join(&archive_name);

    println!("Creating zero-lock hot backup archive: {}...", archive_name);

    // RAII guard will automatically call save-on when dropped
    let _guard = SaveGuard::new(rcon.as_deref_mut());

    let tar_gz = File::create(&archive_path)
        .map_err(|e| format!("Failed to create archive file: {}", e))?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(enc);

    // Recursively append world directory while excluding session.lock
    append_dir_filtered(&mut tar, &world_dir, Path::new(world_name))?;
    tar.finish().map_err(|e| format!("Failed to finalize archive: {}", e))?;

    let size_bytes = fs::metadata(&archive_path)
        .map_err(|e| e.to_string())?
        .len();
    let archive_size_mb = size_bytes / 1024 / 1024;

    println!(
        "Backup successfully created: {} ({} MB)",
        archive_name, archive_size_mb
    );

    // Prune old archives
    let purged_count = prune_old_backups(&out_dir, world_name, retention_days);

    Ok(BackupResult {
        archive_path,
        archive_size_mb,
        purged_count,
    })
}

fn append_dir_filtered<W: std::io::Write>(
    tar: &mut Builder<W>,
    src_dir: &Path,
    dest_path: &Path,
) -> Result<(), String> {
    for entry in fs::read_dir(src_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip Minecraft runtime exclusive lock file and temporary files
        if file_name_str == "session.lock" || file_name_str.ends_with(".tmp") {
            continue;
        }

        let new_dest = dest_path.join(file_name);
        if path.is_dir() {
            append_dir_filtered(tar, &path, &new_dest)?;
        } else {
            tar.append_path_with_name(&path, &new_dest)
                .map_err(|e| format!("Failed to archive file {:?}: {}", path, e))?;
        }
    }
    Ok(())
}

fn prune_old_backups(backup_dir: &Path, world_name: &str, retention_days: u32) -> usize {
    if retention_days == 0 {
        return 0;
    }
    let max_age = Duration::from_secs(retention_days as u64 * 86400);
    let now = SystemTime::now();
    let mut pruned = 0;

    let prefix = format!("{}_", world_name);
    if let Ok(entries) = fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix) && (name.ends_with(".tar.gz") || name.ends_with(".zip")) {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(age) = now.duration_since(modified) {
                                if age > max_age {
                                    if fs::remove_file(&path).is_ok() {
                                        println!("Cleaned up expired archive: {}", name);
                                        pruned += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    pruned
}
