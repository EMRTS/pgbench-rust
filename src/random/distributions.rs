// Statistical distributions for pgbench
// Reference: original-source/pgbench.c lines 1140-1266

use crate::random::prng::Xoroshiro128StarStar;
use rand::RngCore;

// Constants matching original pgbench
pub const MIN_GAUSSIAN_PARAM: f64 = 2.0;
pub const MIN_ZIPFIAN_PARAM: f64 = 1.001;
pub const MAX_ZIPFIAN_PARAM: f64 = 1000.0;

/// Generate a uniformly distributed random integer in the range [min, max] inclusive.
///
/// This is the basic building block for all other distributions.
pub fn random_uniform(rng: &mut Xoroshiro128StarStar, min: i64, max: i64) -> i64 {
    if min > max {
        // Swap if inverted
        return random_uniform(rng, max, min);
    }

    if min == max {
        return min;
    }

    rng.gen_range(min, max)
}

/// Generate an exponentially distributed random integer in the range [min, max] inclusive.
///
/// Uses inverse transform sampling with the given parameter (lambda).
/// The parameter must be > 0.
///
/// Algorithm:
/// 1. Generate uniform random value in (0, 1]
/// 2. Apply inverse exponential CDF: -ln(uniform) / lambda
/// 3. Map to [min, max] range
///
/// Reference: pgbench.c getExponentialRand()
pub fn random_exponential(
    rng: &mut Xoroshiro128StarStar,
    min: i64,
    max: i64,
    parameter: f64,
) -> i64 {
    assert!(parameter > 0.0, "exponential parameter must be > 0");

    if min > max {
        return random_exponential(rng, max, min, parameter);
    }

    if min == max {
        return min;
    }

    let cut = (-parameter).exp();

    // Generate uniform in (0, 1] by inverting [0, 1)
    let uniform = 1.0 - rng.gen_double();

    // Apply inverse exponential transform
    // Result is in [0, 1) when uniform is in (0, 1]
    assert!((1.0 - cut) != 0.0);
    let rand = -(cut + (1.0 - cut) * uniform).ln() / parameter;

    // Map to [min, max] range
    min + ((max - min + 1) as f64 * rand) as i64
}

/// Generate a normally distributed random integer in the range [min, max] inclusive.
///
/// Uses Box-Muller transform with rejection sampling.
/// The parameter controls the standard deviation (must be >= MIN_GAUSSIAN_PARAM).
///
/// Algorithm:
/// 1. Generate standard normal random values using Box-Muller
/// 2. Reject values outside [-parameter, parameter]
/// 3. Normalize to [0, 1) and map to [min, max]
///
/// Reference: pgbench.c getGaussianRand()
pub fn random_gaussian(
    rng: &mut Xoroshiro128StarStar,
    min: i64,
    max: i64,
    parameter: f64,
) -> i64 {
    assert!(
        parameter >= MIN_GAUSSIAN_PARAM,
        "gaussian parameter must be >= {}",
        MIN_GAUSSIAN_PARAM
    );

    if min > max {
        return random_gaussian(rng, max, min, parameter);
    }

    if min == max {
        return min;
    }

    // Get normally-distributed random number in the range [-parameter, parameter)
    // using rejection sampling
    let stdev = loop {
        let val = prng_double_normal(rng);
        if val >= -parameter && val < parameter {
            break val;
        }
    };

    // Normalize stdev from [-parameter, parameter) to [0, 1)
    let rand = (stdev + parameter) / (parameter * 2.0);

    // Map to [min, max] range
    min + ((max - min + 1) as f64 * rand) as i64
}

/// Generate a Zipfian distributed random integer in the range [min, max] inclusive.
///
/// Uses rejection method based on Luc Devroye's algorithm.
/// The s parameter controls the distribution shape (must be in [MIN_ZIPFIAN_PARAM, MAX_ZIPFIAN_PARAM]).
///
/// This works for s > 1.0 but may perform badly for s very close to 1.0.
///
/// Reference: pgbench.c getZipfianRand() and computeIterativeZipfian()
/// Algorithm from: "Non-Uniform Random Variate Generation", Luc Devroye, p. 550-551, Springer 1986
pub fn random_zipfian(
    rng: &mut Xoroshiro128StarStar,
    min: i64,
    max: i64,
    s: f64,
) -> i64 {
    assert!(
        s >= MIN_ZIPFIAN_PARAM && s <= MAX_ZIPFIAN_PARAM,
        "zipfian parameter must be in [{}, {}]",
        MIN_ZIPFIAN_PARAM,
        MAX_ZIPFIAN_PARAM
    );

    if min > max {
        return random_zipfian(rng, max, min, s);
    }

    if min == max {
        return min;
    }

    let n = max - min + 1;
    let value = compute_iterative_zipfian(rng, n, s);

    min - 1 + value
}

/// Compute Zipfian random value using rejection method.
///
/// This is the core Zipfian algorithm separated for clarity.
/// Returns a value in [1, n].
fn compute_iterative_zipfian(rng: &mut Xoroshiro128StarStar, n: i64, s: f64) -> i64 {
    // Ensure n is sane
    if n <= 1 {
        return 1;
    }

    let b = 2.0_f64.powf(s - 1.0);

    loop {
        // Generate two random variates
        let u = rng.gen_double();
        let v = rng.gen_double();

        let x = (u.powf(-1.0 / (s - 1.0))).floor();

        let t = (1.0 + 1.0 / x).powf(s - 1.0);

        // Accept if condition is met and x is in valid range
        if v * x * (t - 1.0) / (b - 1.0) <= t / b && x <= n as f64 {
            return x as i64;
        }
    }
}

/// Generate a standard normal (Gaussian) random value using Box-Muller transform.
///
/// Returns a value from the standard normal distribution (mean=0, stddev=1).
///
/// Box-Muller transform:
/// Given two independent uniform random values U1, U2 in [0, 1):
/// Z0 = sqrt(-2 * ln(U1)) * cos(2 * pi * U2)
/// Z1 = sqrt(-2 * ln(U1)) * sin(2 * pi * U2)
///
/// Both Z0 and Z1 are independent standard normal random values.
/// We use Z0 and cache Z1 for the next call (standard optimization).
fn prng_double_normal(rng: &mut Xoroshiro128StarStar) -> f64 {
    // Box-Muller transform
    // Note: This is a simplified version without caching for thread safety
    // The original PostgreSQL implementation caches one value, but that requires
    // mutable state which complicates the RNG interface.

    let u1 = loop {
        let val = rng.gen_double();
        if val > 0.0 {
            break val;
        }
        // Retry if we get exactly 0.0 (extremely rare) to avoid ln(0)
    };

    let u2 = rng.gen_double();

    // Box-Muller: Z0 = sqrt(-2 * ln(U1)) * cos(2 * pi * U2)
    let magnitude = (-2.0 * u1.ln()).sqrt();
    let angle = 2.0 * std::f64::consts::PI * u2;

    magnitude * angle.cos()

    // Note: We could also generate Z1 = magnitude * angle.sin()
    // and cache it for the next call, but this requires mutable state.
    // For simplicity and thread-safety, we generate a fresh pair each time.
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_uniform_basic() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Test basic range
        for _ in 0..100 {
            let val = random_uniform(&mut rng, 1, 10);
            assert!(val >= 1 && val <= 10, "value {} out of range [1, 10]", val);
        }
    }

    #[test]
    fn test_uniform_single_value() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Single value range
        let val = random_uniform(&mut rng, 5, 5);
        assert_eq!(val, 5);
    }

    #[test]
    fn test_uniform_inverted_range() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Inverted range (max < min) should swap
        let val = random_uniform(&mut rng, 10, 1);
        assert!(val >= 1 && val <= 10);
    }

    #[test]
    fn test_exponential_basic() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Test exponential distribution
        for _ in 0..100 {
            let val = random_exponential(&mut rng, 1, 100, 2.0);
            assert!(val >= 1 && val <= 100, "value {} out of range", val);
        }
    }

    #[test]
    fn test_exponential_parameter_validation() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Small parameter (lambda)
        let val = random_exponential(&mut rng, 1, 100, 0.5);
        assert!(val >= 1 && val <= 100);

        // Large parameter
        let val = random_exponential(&mut rng, 1, 100, 10.0);
        assert!(val >= 1 && val <= 100);
    }

    #[test]
    #[should_panic(expected = "exponential parameter must be > 0")]
    fn test_exponential_invalid_parameter() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);
        random_exponential(&mut rng, 1, 100, 0.0);
    }

    #[test]
    fn test_gaussian_basic() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Test Gaussian distribution with minimum parameter
        for _ in 0..100 {
            let val = random_gaussian(&mut rng, 1, 100, MIN_GAUSSIAN_PARAM);
            assert!(val >= 1 && val <= 100, "value {} out of range", val);
        }
    }

    #[test]
    fn test_gaussian_parameter_validation() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Valid parameters
        let val = random_gaussian(&mut rng, 1, 100, 2.0);
        assert!(val >= 1 && val <= 100);

        let val = random_gaussian(&mut rng, 1, 100, 5.0);
        assert!(val >= 1 && val <= 100);
    }

    #[test]
    #[should_panic(expected = "gaussian parameter must be >= 2")]
    fn test_gaussian_invalid_parameter() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);
        random_gaussian(&mut rng, 1, 100, 1.0);
    }

    #[test]
    fn test_zipfian_basic() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Test Zipfian distribution
        for _ in 0..100 {
            let val = random_zipfian(&mut rng, 1, 100, 1.5);
            assert!(val >= 1 && val <= 100, "value {} out of range", val);
        }
    }

    #[test]
    fn test_zipfian_parameter_range() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Minimum parameter
        let val = random_zipfian(&mut rng, 1, 100, MIN_ZIPFIAN_PARAM);
        assert!(val >= 1 && val <= 100);

        // Mid-range parameter
        let val = random_zipfian(&mut rng, 1, 100, 2.0);
        assert!(val >= 1 && val <= 100);

        // Maximum parameter
        let val = random_zipfian(&mut rng, 1, 100, MAX_ZIPFIAN_PARAM);
        assert!(val >= 1 && val <= 100);
    }

    #[test]
    #[should_panic(expected = "zipfian parameter must be in [1.001, 1000]")]
    fn test_zipfian_invalid_parameter_low() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);
        random_zipfian(&mut rng, 1, 100, 1.0);
    }

    #[test]
    #[should_panic(expected = "zipfian parameter must be in [1.001, 1000]")]
    fn test_zipfian_invalid_parameter_high() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);
        random_zipfian(&mut rng, 1, 100, 1001.0);
    }

    #[test]
    fn test_prng_double_normal() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Generate many samples and check they look reasonable
        let mut values = Vec::new();
        for _ in 0..1000 {
            let val = prng_double_normal(&mut rng);
            values.push(val);
        }

        // Check mean is close to 0 (within 0.1)
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        assert!(
            mean.abs() < 0.1,
            "mean {} too far from 0 for normal distribution",
            mean
        );

        // Check most values are within 3 standard deviations
        let within_3_sigma = values.iter().filter(|&&v| v.abs() <= 3.0).count();
        let ratio = within_3_sigma as f64 / values.len() as f64;
        assert!(
            ratio > 0.99,
            "only {:.2}% within 3 sigma, expected >99%",
            ratio * 100.0
        );
    }

    #[test]
    fn test_distribution_reproducibility() {
        // Same seed should produce same sequence
        let mut rng1 = Xoroshiro128StarStar::seed_from_u64(12345);
        let mut rng2 = Xoroshiro128StarStar::seed_from_u64(12345);

        for _ in 0..10 {
            assert_eq!(
                random_uniform(&mut rng1, 1, 100),
                random_uniform(&mut rng2, 1, 100)
            );
        }

        for _ in 0..10 {
            assert_eq!(
                random_exponential(&mut rng1, 1, 100, 2.0),
                random_exponential(&mut rng2, 1, 100, 2.0)
            );
        }

        for _ in 0..10 {
            assert_eq!(
                random_gaussian(&mut rng1, 1, 100, 3.0),
                random_gaussian(&mut rng2, 1, 100, 3.0)
            );
        }

        for _ in 0..10 {
            assert_eq!(
                random_zipfian(&mut rng1, 1, 100, 1.5),
                random_zipfian(&mut rng2, 1, 100, 1.5)
            );
        }
    }

    #[test]
    fn test_zipfian_edge_cases() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Small range
        let val = random_zipfian(&mut rng, 1, 2, 1.5);
        assert!(val >= 1 && val <= 2);

        // Single value
        let val = random_zipfian(&mut rng, 5, 5, 1.5);
        assert_eq!(val, 5);
    }

    #[test]
    fn test_distributions_coverage() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(999);

        // Generate many samples to ensure we hit different parts of the range
        let mut uniform_vals = std::collections::HashSet::new();
        let mut exp_vals = std::collections::HashSet::new();
        let mut gauss_vals = std::collections::HashSet::new();
        let mut zipf_vals = std::collections::HashSet::new();

        for _ in 0..1000 {
            uniform_vals.insert(random_uniform(&mut rng, 1, 10));
            exp_vals.insert(random_exponential(&mut rng, 1, 10, 2.0));
            gauss_vals.insert(random_gaussian(&mut rng, 1, 10, 3.0));
            zipf_vals.insert(random_zipfian(&mut rng, 1, 10, 1.5));
        }

        // Each distribution should hit multiple values
        assert!(uniform_vals.len() >= 8, "uniform didn't cover enough values");
        assert!(exp_vals.len() >= 5, "exponential didn't cover enough values");
        assert!(gauss_vals.len() >= 5, "gaussian didn't cover enough values");
        assert!(zipf_vals.len() >= 5, "zipfian didn't cover enough values");
    }
}
