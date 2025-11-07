//! Random number generation
//!
//! This module provides random number generation compatible with pgbench,
//! including various statistical distributions.
//!
//! The PRNG uses the Xoroshiro128** algorithm, matching PostgreSQL's pg_prng.
//! The distributions are implemented to match the original pgbench C implementation exactly.

pub mod prng;
pub mod distributions;

pub use prng::{PgBenchRng, Xoroshiro128StarStar};
pub use distributions::*;
