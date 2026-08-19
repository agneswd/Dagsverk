//! Pure Dagsverk domain logic.

pub mod calculations;
pub mod clock;
pub mod error;
pub mod holidays;
pub mod i18n;
pub mod models;
pub mod tax;

pub use error::{DomainError, Result};
