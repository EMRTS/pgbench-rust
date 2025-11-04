//! Random number generation
//!
//! This module provides random number generation compatible with pgbench,
//! including various statistical distributions.

pub mod prng;
pub mod distributions;

pub use prng::{PgBenchRng, Xoroshiro128StarStar};
pub use distributions::*;

// TODO: Implement uniform distribution
// TODO: Implement Gaussian distribution
// TODO: Implement exponential distribution
// TODO: Implement Zipfian distribution
// TODO: Ensure statistical correctness
