use std::{fs, path::Path};

use dagsverk_core::clock::Clock;
use rusqlite::{Connection, OpenFlags};
use uuid::Uuid;

use crate::{DataError, Database, Result, backup::backup_connection, schema};

impl<C: Clock> Database<C> {
    pub fn restore_backup(&self, selected: &Path) -> Result<()> {
        if !selected.exists() {
            return Err(DataError::MissingFile(selected.to_owned()));
        }
        let backup_folder = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        fs::create_dir_all(&backup_folder).map_err(|source| DataError::CreateDirectory {
            path: backup_folder.clone(),
            source,
        })?;
        let candidate = backup_folder.join(format!(".restore-{}.db", Uuid::new_v4()));
        let result = self.restore_from_candidate(selected, &candidate);
        let _ = fs::remove_file(&candidate);
        result
    }

    fn restore_from_candidate(&self, selected: &Path, candidate: &Path) -> Result<()> {
        let source = Connection::open_with_flags(selected, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        backup_connection(&source, candidate)?;
        drop(source);
        schema::validate_path(candidate)?;
        let safety = self.create_backup(None, "before-restore")?;

        self.remove_sidecars()?;
        if let Err(error) =
            copy(candidate, &self.path).and_then(|()| schema::validate_path(&self.path))
        {
            self.remove_sidecars()?;
            copy(&safety, &self.path)?;
            schema::validate_path(&self.path)?;
            return Err(error);
        }
        Ok(())
    }

    fn remove_sidecars(&self) -> Result<()> {
        for suffix in ["-wal", "-shm"] {
            let path = std::path::PathBuf::from(format!("{}{suffix}", self.path.display()));
            if path.exists() {
                fs::remove_file(&path).map_err(|source| DataError::Io {
                    operation: "remove database sidecar",
                    path,
                    source,
                })?;
            }
        }
        Ok(())
    }
}

fn copy(source: &Path, destination: &Path) -> Result<()> {
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|source_error| DataError::Io {
            operation: "replace database",
            path: destination.to_owned(),
            source: source_error,
        })
}
