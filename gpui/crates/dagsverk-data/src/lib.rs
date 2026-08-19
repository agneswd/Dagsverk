//! SQLite compatibility and data safety services.

mod backup;
mod connection;
mod error;
mod migration;
pub mod paths;
mod repository;
mod restore;
mod schema;
mod tidverk_import;

pub use connection::Database;
pub use error::{DataError, Result};
pub use repository::DagsverkRepository;
pub use tidverk_import::TidverkImportResult;

use std::path::{Path, PathBuf};

/// Runs database-wide maintenance outside the UI and repository layers.
pub trait DataMaintenance: Send + Sync {
    fn database_path(&self) -> PathBuf;
    fn create_manual_backup(&self) -> Result<PathBuf>;
    fn restore(&self, selected: &Path) -> Result<()>;
    fn import_tidverk_database(&self, selected: &Path) -> Result<TidverkImportResult>;
}

impl<C: dagsverk_core::clock::Clock> DataMaintenance for Database<C> {
    fn database_path(&self) -> PathBuf {
        self.path().to_owned()
    }

    fn create_manual_backup(&self) -> Result<PathBuf> {
        self.create_backup(None, "manual")
    }

    fn restore(&self, selected: &Path) -> Result<()> {
        self.restore_backup(selected)
    }

    fn import_tidverk_database(&self, selected: &Path) -> Result<TidverkImportResult> {
        self.import_tidverk(selected)
    }
}
