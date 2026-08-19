use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use dagsverk_core::clock::Clock;
use rusqlite::{Connection, backup::Backup};
use uuid::Uuid;

use crate::{DataError, Database, Result};

const RETAINED_BACKUP_COUNT: usize = 5;

impl<C: Clock> Database<C> {
    pub fn create_backup(&self, destination: Option<&Path>, reason: &str) -> Result<PathBuf> {
        let folder = destination.map(Path::to_owned).unwrap_or_else(|| {
            self.path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("backups")
        });
        fs::create_dir_all(&folder).map_err(|source| DataError::CreateDirectory {
            path: folder.clone(),
            source,
        })?;
        let safe_reason: String = reason
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
            .collect();
        let reason = if safe_reason.is_empty() {
            "backup"
        } else {
            &safe_reason
        };
        let timestamp = self.clock.now_utc().format("%Y-%m-%dT%H-%M-%S-%3fZ");
        let path = folder.join(format!(
            "dagsverk-backup-{timestamp}-{}-{reason}.db",
            Uuid::new_v4()
        ));
        backup_connection(&self.connection()?, &path)?;
        prune_backups(&folder)?;
        Ok(path)
    }
}

pub(crate) fn backup_connection(source: &Connection, destination: &Path) -> Result<()> {
    let mut target = Connection::open(destination)?;
    let backup = Backup::new(source, &mut target)?;
    backup.run_to_completion(64, Duration::from_millis(10), None)?;
    Ok(())
}

pub(crate) fn prune_backups(folder: &Path) -> Result<()> {
    let mut backups = fs::read_dir(folder)
        .map_err(|source| DataError::Io {
            operation: "read backup directory",
            path: folder.to_owned(),
            source,
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("dagsverk-backup-") && name.ends_with(".db"))
        })
        .collect::<Vec<_>>();
    backups.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    for path in backups.into_iter().skip(RETAINED_BACKUP_COUNT) {
        fs::remove_file(&path).map_err(|source| DataError::Io {
            operation: "prune backup",
            path,
            source,
        })?;
    }
    Ok(())
}
