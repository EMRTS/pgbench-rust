//! pgbench-rust library
//!
//! This library provides the core functionality for PostgreSQL benchmarking,
//! ported from the C implementation of pgbench.
//!
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! - `db`: Database connection and operations
//! - `expr`: Expression parser and evaluator
//! - `script`: Transaction script parsing and execution
//! - `random`: Random number generation and distributions
//! - `worker`: Thread management and workload execution
//! - `stats`: Statistics collection and reporting
//! - `types`: Common type definitions
//! - `error`: Error types and handling

pub mod cli;
pub mod db;
pub mod expr;
pub mod script;
pub mod random;
pub mod worker;
pub mod stats;
pub mod types;
pub mod error;
pub mod utils;

// Re-export commonly used types
pub use types::*;
pub use error::PgBenchError;
