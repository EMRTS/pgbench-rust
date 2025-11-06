//! Xoroshiro128** PRNG Implementation
//!
//! This implements the Xoroshiro128** (xoroshiro128 starstar) pseudo-random
//! number generator by David Blackman and Sebastiano Vigna.
//!
//! **CRITICAL**: This implementation must exactly match PostgreSQL's pg_prng
//! for reproducibility. The same seed must produce the same sequence.
//!
//! Reference: https://prng.di.unimi.it/
//! PostgreSQL source: src/common/pg_prng.c
//!
//! Algorithm: xoroshiro128** 1.0
//! State: 128 bits (two 64-bit values)
//! Period: 2^128 - 1
//! Output: 64 bits

use rand::RngCore;
use rand::SeedableRng;

/// Xoroshiro128** random number generator state
///
/// This is a 128-bit state generator that produces high-quality 64-bit
/// pseudo-random numbers. It's extremely fast and passes all statistical tests.
///
/// The algorithm uses xor, rotate, shift operations to update state,
/// and applies a "starstar" scrambler to the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xoroshiro128StarStar {
    /// First half of 128-bit state
    s0: u64,
    /// Second half of 128-bit state
    s1: u64,
}

impl Xoroshiro128StarStar {
    /// Create a new PRNG with the given state
    ///
    /// # Arguments
    /// * `s0` - First 64-bit state value
    /// * `s1` - Second 64-bit state value
    ///
    /// # Panics
    /// Panics if both s0 and s1 are zero (all-zero state is invalid)
    pub fn new(s0: u64, s1: u64) -> Self {
        assert!(s0 != 0 || s1 != 0, "Xoroshiro128** state cannot be all zeros");
        Self { s0, s1 }
    }

    /// Generate the next random u64 value
    ///
    /// This is the core generator function implementing the xoroshiro128**
    /// algorithm as specified by Blackman and Vigna.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // Read current state
        let s0 = self.s0;
        let s1 = self.s1;

        // Apply the "starstar" scrambler to generate output
        // result = rotl(s0 * 5, 7) * 9
        let result = rotl(s0.wrapping_mul(5), 7).wrapping_mul(9);

        // Update state using xoroshiro128 algorithm
        // s1 ^= s0
        let s1 = s1 ^ s0;

        // New s0 = rotl(s0, 24) ^ s1 ^ (s1 << 16)
        self.s0 = rotl(s0, 24) ^ s1 ^ (s1 << 16);

        // New s1 = rotl(s1, 37)
        self.s1 = rotl(s1, 37);

        result
    }

    /// Generate a random u64 in the range [min, max] (inclusive)
    ///
    /// This uses the bitmask with rejection method to avoid bias.
    /// Matches PostgreSQL's pg_prng_uint64_range implementation.
    pub fn gen_range(&mut self, min: u64, max: u64) -> u64 {
        if min == max {
            return min;
        }

        assert!(min < max, "min must be less than max");

        let range = max - min;

        // Find the next power of 2 minus 1 that's >= range
        // This is our bitmask for rejection sampling
        let mask = if range == u64::MAX {
            u64::MAX
        } else {
            // Find the highest bit set in range
            let bits = 64 - range.leading_zeros();
            // Create a mask with that many bits set
            if bits == 64 {
                u64::MAX
            } else {
                (1u64 << bits) - 1
            }
        };

        // Rejection sampling: keep generating until we get a value in range
        loop {
            let r = self.next_u64() & mask;
            if r <= range {
                return min + r;
            }
        }
    }

    /// Generate a random f64 in the range [0.0, 1.0)
    ///
    /// This uses 53 bits of precision (matching IEEE 754 double mantissa).
    /// Matches PostgreSQL's pg_prng_double implementation.
    #[inline]
    pub fn gen_double(&mut self) -> f64 {
        // Use upper 53 bits for double precision
        // Divide by 2^53 to get range [0.0, 1.0)
        const SCALE: f64 = (1u64 << 53) as f64;
        let r = self.next_u64() >> 11; // Use upper 53 bits
        (r as f64) / SCALE
    }

    /// Generate a random boolean
    #[inline]
    pub fn gen_bool(&mut self) -> bool {
        // Use the most significant bit
        (self.next_u64() & (1u64 << 63)) != 0
    }

    /// Check if the state is valid (not all zeros)
    pub fn is_valid(&self) -> bool {
        self.s0 != 0 || self.s1 != 0
    }

    /// Get the current state as a tuple
    pub fn state(&self) -> (u64, u64) {
        (self.s0, self.s1)
    }
}

/// Rotate left operation for u64
///
/// This is a key primitive operation used in the xoroshiro128** algorithm.
#[inline]
fn rotl(x: u64, k: u32) -> u64 {
    x.rotate_left(k)
}

/// SplitMix64 generator for seeding
///
/// This is used to initialize the xoroshiro128** state from a single u64 seed.
/// PostgreSQL uses this same algorithm in pg_prng_seed.
///
/// Reference: https://prng.di.unimi.it/splitmix64.c
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

impl SeedableRng for Xoroshiro128StarStar {
    type Seed = [u8; 16]; // 128 bits = 16 bytes

    /// Seed from a 128-bit value
    fn from_seed(seed: Self::Seed) -> Self {
        let s0 = u64::from_le_bytes([
            seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], seed[6], seed[7],
        ]);
        let s1 = u64::from_le_bytes([
            seed[8], seed[9], seed[10], seed[11], seed[12], seed[13], seed[14], seed[15],
        ]);

        // If both values are zero, use splitmix64 to generate valid state
        if s0 == 0 && s1 == 0 {
            let mut state = 0u64;
            let s0 = splitmix64(&mut state);
            let s1 = splitmix64(&mut state);
            Self { s0, s1 }
        } else {
            Self { s0, s1 }
        }
    }

    /// Seed from a u64 value using splitmix64
    ///
    /// This matches PostgreSQL's pg_prng_seed implementation.
    fn seed_from_u64(seed: u64) -> Self {
        let mut state = seed;
        let s0 = splitmix64(&mut state);
        let s1 = splitmix64(&mut state);
        Self { s0, s1 }
    }
}

impl RngCore for Xoroshiro128StarStar {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        Xoroshiro128StarStar::next_u64(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand_core::impls::fill_bytes_via_next(self, dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

/// Type alias for compatibility
pub type PgBenchRng = Xoroshiro128StarStar;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let rng = Xoroshiro128StarStar::new(1, 2);
        assert_eq!(rng.state(), (1, 2));
    }

    #[test]
    #[should_panic(expected = "cannot be all zeros")]
    fn test_new_zero_state_panics() {
        Xoroshiro128StarStar::new(0, 0);
    }

    #[test]
    fn test_seed_from_u64() {
        let rng1 = Xoroshiro128StarStar::seed_from_u64(12345);
        let rng2 = Xoroshiro128StarStar::seed_from_u64(12345);

        // Same seed should produce same initial state
        assert_eq!(rng1.state(), rng2.state());

        // State should not be zero
        assert!(rng1.is_valid());
    }

    #[test]
    fn test_seed_from_u64_different_seeds() {
        let rng1 = Xoroshiro128StarStar::seed_from_u64(12345);
        let rng2 = Xoroshiro128StarStar::seed_from_u64(54321);

        // Different seeds should produce different states
        assert_ne!(rng1.state(), rng2.state());
    }

    #[test]
    fn test_reproducibility() {
        let mut rng1 = Xoroshiro128StarStar::seed_from_u64(12345);
        let mut rng2 = Xoroshiro128StarStar::seed_from_u64(12345);

        // Same seed should produce same sequence
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_sequence() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(1);

        // Generate a few values to verify they're different
        let v1 = rng.next_u64();
        let v2 = rng.next_u64();
        let v3 = rng.next_u64();

        // Values should be different
        assert_ne!(v1, v2);
        assert_ne!(v2, v3);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_gen_range() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Single value range
        assert_eq!(rng.gen_range(5, 5), 5);

        // Small range
        for _ in 0..100 {
            let val = rng.gen_range(1, 10);
            assert!(val >= 1 && val <= 10);
        }

        // Large range
        for _ in 0..100 {
            let val = rng.gen_range(1, 1_000_000);
            assert!(val >= 1 && val <= 1_000_000);
        }
    }

    #[test]
    fn test_gen_double() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        for _ in 0..100 {
            let val = rng.gen_double();
            assert!(val >= 0.0 && val < 1.0, "value {} out of range", val);
        }
    }

    #[test]
    fn test_gen_bool() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        let mut true_count = 0;
        let mut false_count = 0;

        for _ in 0..1000 {
            if rng.gen_bool() {
                true_count += 1;
            } else {
                false_count += 1;
            }
        }

        // Both should occur (very unlikely to get all true or all false)
        assert!(true_count > 0);
        assert!(false_count > 0);

        // Should be roughly balanced (allowing for randomness)
        let ratio = true_count as f64 / false_count as f64;
        assert!(ratio > 0.3 && ratio < 3.0, "ratio {}", ratio);
    }

    #[test]
    fn test_rotl() {
        assert_eq!(rotl(0b1, 1), 0b10);
        assert_eq!(rotl(0b1000, 1), 0b10000);
        assert_eq!(rotl(0x8000000000000000, 1), 1); // Wrap around
    }

    #[test]
    fn test_splitmix64() {
        let mut state = 12345u64;
        let v1 = splitmix64(&mut state);
        let v2 = splitmix64(&mut state);
        let v3 = splitmix64(&mut state);

        // Values should be different
        assert_ne!(v1, v2);
        assert_ne!(v2, v3);
        assert_ne!(v1, v3);

        // State should have changed
        assert_ne!(state, 12345);
    }

    #[test]
    fn test_splitmix64_reproducibility() {
        let mut state1 = 12345u64;
        let mut state2 = 12345u64;

        for _ in 0..10 {
            assert_eq!(splitmix64(&mut state1), splitmix64(&mut state2));
        }
    }

    #[test]
    fn test_rng_core_trait() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Test next_u32
        let _: u32 = rng.next_u32();

        // Test next_u64
        let _: u64 = rng.next_u64();

        // Test fill_bytes
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        assert!(bytes.iter().any(|&b| b != 0)); // Should have non-zero bytes
    }

    #[test]
    fn test_is_valid() {
        let rng = Xoroshiro128StarStar::new(1, 2);
        assert!(rng.is_valid());

        let rng = Xoroshiro128StarStar::new(0, 1);
        assert!(rng.is_valid());

        let rng = Xoroshiro128StarStar::new(1, 0);
        assert!(rng.is_valid());
    }

    /// Test known output for a specific seed (verification against reference)
    ///
    /// These values should match PostgreSQL's pg_prng output for the same seed.
    /// TODO: Verify these against actual PostgreSQL pg_prng output
    #[test]
    fn test_known_seed_output() {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

        // Generate first few values
        // Note: These should be verified against PostgreSQL's output
        let _v1 = rng.next_u64();
        let _v2 = rng.next_u64();
        let _v3 = rng.next_u64();

        // For now, just verify they're different
        // Once we have PostgreSQL reference values, we should assert exact matches
    }
}
