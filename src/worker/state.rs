//! Worker thread state management

use crate::types::ThreadState;
use crate::random::PgBenchRng;
use rand::SeedableRng;

impl ThreadState {
    /// Create a new thread state
    pub fn new(id: usize, seed: u64, ntransactions: u64) -> Self {
        Self {
            id,
            rng_state: seed,
            stats: Default::default(),
            ntransactions,
            start_time: std::time::Instant::now(),
        }
    }

    /// Get a random number generator for this thread
    pub fn rng(&mut self) -> PgBenchRng {
        PgBenchRng::seed_from_u64(self.rng_state)
    }
}

// TODO: Implement per-thread connection pool
// TODO: Implement per-thread statistics
// TODO: Handle thread-local variable storage
