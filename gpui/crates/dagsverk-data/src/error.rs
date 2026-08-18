use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Domain(#[from] dagsverk_core::DomainError),
    #[error("failed to create data directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid value in column {column}: {value}")]
    InvalidValue { column: &'static str, value: String },
    #[error("failed to serialize compensation rules: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid decimal in column {column}: {value}")]
    Decimal { column: &'static str, value: String },
    #[error("invalid timestamp in column {column}: {value}")]
    Timestamp { column: &'static str, value: String },
    #[error("cannot delete the last remaining workspace")]
    LastWorkspace,
    #[error("SQLite integrity validation failed: {0}")]
    Integrity(String),
    #[error("the selected file is not a Dagsverk database")]
    NotDagsverkDatabase,
}

pub type Result<T> = std::result::Result<T, DataError>;
