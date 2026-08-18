use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("the report month is invalid")]
    InvalidMonth,
    #[error("{0} is outside the selected month")]
    EntryOutsideMonth(String),
    #[error("the report contains duplicate entries for {0}")]
    DuplicateDate(String),
    #[error("the worked time for {0} is invalid")]
    InvalidTime(String),
    #[error("the lunch duration for {0} is invalid")]
    InvalidLunch(String),
    #[error("the output file must use the .{expected} extension: {path}")]
    InvalidExtension {
        expected: &'static str,
        path: PathBuf,
    },
    #[error("failed to write {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("XLSX generation failed: {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
    #[error("ODS generation failed: {0}")]
    Zip(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, ExportError>;
