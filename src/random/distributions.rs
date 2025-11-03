//! Statistical distributions for random number generation

use crate::error::{PgBenchError, PgBenchResult};
use crate::random::PgBenchRng;
use rand::RngCore;
use rand_distr::{Distribution, Normal, Exp};

/// Generate a uniform random integer in [min, max]
pub fn random_uniform(rng: &mut PgBenchRng, min: i64, max: i64) -> i64 {
    if min >= max {
        return min;
    }

    let range = (max - min + 1) as u64;
    let value = rng.next_u64() % range;
    min + value as i64
}

/// Generate a Gaussian (normal) distributed random number
pub fn random_gaussian(
    rng: &mut PgBenchRng,
    min: i64,
    max: i64,
    param: f64,
) -> PgBenchResult<i64> {
    if param < 2.0 {
        return Err(PgBenchError::RandomError(
            "Gaussian parameter must be at least 2.0".to_string(),
        ));
    }

    let mean = (min + max) as f64 / 2.0;
    let stddev = (max - min) as f64 / param;

    let normal = Normal::new(mean, stddev)
        .map_err(|e| PgBenchError::RandomError(e.to_string()))?;

    let value = normal.sample(rng);
    Ok(value.round() as i64)
}

/// Generate an exponentially distributed random number
pub fn random_exponential(
    rng: &mut PgBenchRng,
    min: i64,
    max: i64,
    param: f64,
) -> PgBenchResult<i64> {
    let exp = Exp::new(param)
        .map_err(|e| PgBenchError::RandomError(e.to_string()))?;

    let value = exp.sample(rng);
    let scaled = min as f64 + value * (max - min) as f64;
    Ok(scaled.round() as i64)
}

/// Generate a Zipfian distributed random number
pub fn random_zipfian(
    _rng: &mut PgBenchRng,
    _min: i64,
    _max: i64,
    _param: f64,
) -> PgBenchResult<i64> {
    // TODO: Implement Zipfian distribution
    // This is a complex distribution that requires careful implementation
    Err(PgBenchError::RandomError(
        "Zipfian distribution not yet implemented".to_string(),
    ))
}

// TODO: Implement Zipfian distribution algorithm
// TODO: Verify all distributions match pgbench behavior
// TODO: Add statistical tests
