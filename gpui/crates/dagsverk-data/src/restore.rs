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

        replace_with_rollback(
            candidate,
            &self.path,
            &safety,
            |source, destination| {
                self.remove_sidecars()?;
                copy(source, destination)
            },
            schema::validate_path,
        )
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

fn replace_with_rollback(
    candidate: &Path,
    current: &Path,
    safety: &Path,
    mut replace: impl FnMut(&Path, &Path) -> Result<()>,
    validate: impl Fn(&Path) -> Result<()>,
) -> Result<()> {
    if let Err(error) = replace(candidate, current).and_then(|()| validate(current)) {
        replace(safety, current)?;
        validate(current)?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::replace_with_rollback;
    use crate::{DataError, Result};

    #[test]
    fn failed_replacement_restores_the_safety_copy() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let current = directory.path().join("current");
        let candidate = directory.path().join("candidate");
        let safety = directory.path().join("safety");
        fs::write(&current, b"original").expect("current");
        fs::write(&candidate, b"invalid").expect("candidate");
        fs::write(&safety, b"original").expect("safety");

        let result = replace_with_rollback(
            &candidate,
            &current,
            &safety,
            |source, destination| {
                fs::copy(source, destination)
                    .map(|_| ())
                    .map_err(|error| DataError::Io {
                        operation: "test replace",
                        path: destination.to_owned(),
                        source: error,
                    })
            },
            |path| -> Result<()> {
                if fs::read(path).expect("read") == b"original" {
                    Ok(())
                } else {
                    Err(DataError::Integrity("test failure".to_owned()))
                }
            },
        );

        assert!(matches!(result, Err(DataError::Integrity(_))));
        assert_eq!(fs::read(current).expect("restored file"), b"original");
    }
}
