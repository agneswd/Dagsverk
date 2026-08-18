//! SQLite compatibility and data safety services.

mod connection;
mod error;
pub mod paths;
mod schema;

pub use connection::Database;
pub use error::{DataError, Result};
