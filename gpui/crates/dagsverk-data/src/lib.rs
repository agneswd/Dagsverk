//! SQLite compatibility and data safety services.

mod backup;
mod connection;
mod error;
mod migration;
pub mod paths;
mod restore;
mod schema;

pub use connection::Database;
pub use error::{DataError, Result};
