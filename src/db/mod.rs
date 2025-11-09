//! Database operations module
//!
//! This module handles all database connectivity and operations,
//! including initialization, query execution, and connection management.

pub mod connection;
pub mod init;
pub mod query;

pub use connection::PgBenchConnection;
pub use init::detect_scale_factor;
