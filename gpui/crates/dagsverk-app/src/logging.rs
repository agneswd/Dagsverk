use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, SystemTime},
};

use chrono::Utc;

const MAX_LOG_BYTES: u64 = 1_000_000;
const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

static LOG_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

/// Starts file logging beside the active database. Logging failures never stop startup.
pub fn initialize(database_path: &Path) {
    let Some(data_directory) = database_path.parent() else {
        return;
    };
    let directory = data_directory.join("logs");
    if prepare(&directory, SystemTime::now()).is_ok() {
        let _ = LOG_DIRECTORY.set(directory);
    }
}

pub fn info(message: &str) {
    write("INFO", message, None);
}

pub fn error(message: &str, detail: &dyn std::fmt::Display) {
    write("ERROR", message, Some(detail));
}

fn write(level: &str, message: &str, detail: Option<&dyn std::fmt::Display>) {
    let Some(directory) = LOG_DIRECTORY.get() else {
        return;
    };
    let now = Utc::now();
    let path = directory.join(format!("dagsverk-gpui-{}.log", now.format("%Y-%m-%d")));
    if rotate(&path).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = match detail {
        Some(detail) => writeln!(file, "{} {level} {message} {detail}", now.to_rfc3339()),
        None => writeln!(file, "{} {level} {message}", now.to_rfc3339()),
    };
}

fn prepare(directory: &Path, now: SystemTime) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("dagsverk-gpui-") || !name.contains(".log") {
            continue;
        }
        let Ok(modified) = entry.metadata()?.modified() else {
            continue;
        };
        if now
            .duration_since(modified)
            .is_ok_and(|age| age > RETENTION)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

fn rotate(path: &Path) -> std::io::Result<()> {
    if !path
        .metadata()
        .is_ok_and(|metadata| metadata.len() > MAX_LOG_BYTES)
    {
        return Ok(());
    }
    let old = path.with_extension("log.old");
    if old.exists() {
        fs::remove_file(&old)?;
    }
    fs::rename(path, old)
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use tempfile::tempdir;

    use super::{MAX_LOG_BYTES, prepare, rotate};

    #[test]
    fn rotates_a_large_log_and_ignores_unrelated_files() {
        let directory = tempdir().expect("temporary log directory");
        let log = directory.path().join("dagsverk-gpui-2026-08-18.log");
        fs::write(&log, vec![0; MAX_LOG_BYTES as usize + 1]).expect("large log");
        fs::write(directory.path().join("keep.txt"), "keep").expect("unrelated file");

        prepare(directory.path(), SystemTime::now()).expect("prepare logs");
        rotate(&log).expect("rotate log");

        assert!(!log.exists());
        assert!(
            directory
                .path()
                .join("dagsverk-gpui-2026-08-18.log.old")
                .exists()
        );
        assert!(directory.path().join("keep.txt").exists());
    }
}
