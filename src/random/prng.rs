//! Pseudo-random number generator
//!
//! Implements a PRNG compatible with PostgreSQL's pg_prng.
//! Uses Xoroshiro128** algorithm for compatibility and reproducibility.

use rand::{RngCore, SeedableRng};

/// pgbench-compatible random number generator
pub struct PgBenchRng {
    state: [u64; 2],
}

impl PgBenchRng {
    /// Create a new RNG with the given seed
    pub fn new(seed: u64) -> Self {
        // TODO: Implement proper seeding to match pg_prng
        let mut rng = Self {
            state: [seed, seed.wrapping_add(0x9e3779b97f4a7c15)],
        };

        // Warm up the generator
        for _ in 0..10 {
            rng.next_u64();
        }

        rng
    }

    /// Generate a random double in [0.0, 1.0)
    pub fn next_double(&mut self) -> f64 {
        let bits = self.next_u64();
        // Convert to double in [0, 1)
        (bits >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl RngCore for PgBenchRng {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        // TODO: Implement Xoroshiro128** algorithm
        // This is a placeholder implementation
        let s0 = self.state[0];
        let mut s1 = self.state[1];
        let result = s0.wrapping_add(s1);

        s1 ^= s0;
        self.state[0] = s0.rotate_left(24) ^ s1 ^ (s1 << 16);
        self.state[1] = s1.rotate_left(37);

        result
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(8) {
            let val = self.next_u64();
            let bytes = val.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl SeedableRng for PgBenchRng {
    type Seed = [u8; 16];

    fn from_seed(seed: Self::Seed) -> Self {
        let s0 = u64::from_le_bytes([
            seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], seed[6], seed[7],
        ]);
        let s1 = u64::from_le_bytes([
            seed[8], seed[9], seed[10], seed[11], seed[12], seed[13], seed[14], seed[15],
        ]);

        let mut rng = Self { state: [s0, s1] };

        // Warm up
        for _ in 0..10 {
            rng.next_u64();
        }

        rng
    }
}

// TODO: Verify exact algorithm matches PostgreSQL's pg_prng.c
// TODO: Add tests to ensure reproducibility
// TODO: Add statistical tests
