use thiserror::Error;

pub type Result<T> = std::result::Result<T, DomainError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("{enum_name} has unknown persisted value {value}")]
    UnknownEnumValue { enum_name: &'static str, value: i64 },
    #[error("workspace ID must not be empty")]
    EmptyWorkspaceId,
    #[error("project ID must not be empty")]
    EmptyProjectId,
    #[error("invalid ISO date: {0}")]
    InvalidIsoDate(String),
    #[error("invalid clock time: {0}")]
    InvalidClockTime(String),
    #[error("invalid year and month: {year}-{month}")]
    InvalidYearMonth { year: i32, month: u32 },
}
