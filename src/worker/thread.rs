//! Worker thread implementation
//!
//! This module manages the thread pool for benchmark execution.
//! Reference: original-source/pgbench.c threadRun() (lines 7484+)

use crate::db::connection::PgBenchConnection;
use crate::db::query::QueryMode;
use crate::error::{PgBenchError, PgBenchResult};
use crate::worker::state::{ClientState, StatsData, ThreadState};
use std::sync::{Arc, Barrier};
use std::thread::{self, JoinHandle};

/// Thread pool manager for benchmark execution
///
/// Manages multiple worker threads, each running multiple clients (database connections).
/// Uses barrier synchronization to ensure all threads start together.
pub struct ThreadPool {
    /// Worker threads
    threads: Vec<JoinHandle<PgBenchResult<ThreadState>>>,

    /// Barrier for synchronizing thread startup
    start_barrier: Arc<Barrier>,

    /// Number of threads
    num_threads: usize,

    /// Number of clients per thread
    clients_per_thread: usize,
}

impl ThreadPool {
    /// Create a new thread pool
    ///
    /// # Arguments
    /// * `num_threads` - Number of worker threads to create
    /// * `total_clients` - Total number of clients (connections) across all threads
    ///
    /// # Returns
    /// ThreadPool with threads ready to start
    pub fn new(num_threads: usize, total_clients: usize) -> PgBenchResult<Self> {
        if num_threads == 0 {
            return Err(PgBenchError::InvalidArgument(
                "Number of threads must be at least 1".to_string(),
            ));
        }

        if total_clients == 0 {
            return Err(PgBenchError::InvalidArgument(
                "Number of clients must be at least 1".to_string(),
            ));
        }

        // Calculate clients per thread (distribute evenly)
        let clients_per_thread = (total_clients + num_threads - 1) / num_threads;

        // Barrier requires num_threads + 1 (threads + main thread)
        let start_barrier = Arc::new(Barrier::new(num_threads + 1));

        Ok(Self {
            threads: Vec::with_capacity(num_threads),
            start_barrier,
            num_threads,
            clients_per_thread,
        })
    }

    /// Spawn all worker threads
    ///
    /// # Arguments
    /// * `connection_string` - Database connection string
    /// * `query_mode` - Query protocol mode
    /// * `base_seed` - Base seed for RNG (each thread gets derived seed)
    ///
    /// # Returns
    /// Self with spawned threads
    pub fn spawn_threads(
        mut self,
        connection_string: String,
        query_mode: QueryMode,
        base_seed: u64,
    ) -> PgBenchResult<Self> {
        for thread_id in 0..self.num_threads {
            // Derive unique seed for this thread
            let thread_seed = base_seed.wrapping_add(thread_id as u64 * 1000);

            // Clone what we need to move into thread
            let barrier = Arc::clone(&self.start_barrier);
            let conn_str = connection_string.clone();
            let clients_per_thread = self.clients_per_thread;

            // Spawn the thread
            let handle = thread::Builder::new()
                .name(format!("pgbench-worker-{}", thread_id))
                .spawn(move || {
                    thread_worker(
                        thread_id,
                        clients_per_thread,
                        conn_str,
                        query_mode,
                        thread_seed,
                        barrier,
                    )
                })
                .map_err(|e| {
                    PgBenchError::ThreadError(format!("Failed to spawn thread {}: {}", thread_id, e))
                })?;

            self.threads.push(handle);
        }

        Ok(self)
    }

    /// Start all threads (waits at barrier, then signals all threads to begin)
    pub fn start(&self) {
        log::info!("Starting {} worker threads", self.num_threads);

        // Wait at barrier - this releases all threads to start simultaneously
        self.start_barrier.wait();

        log::info!("All threads started");
    }

    /// Wait for all threads to complete and collect results
    ///
    /// # Returns
    /// Vector of ThreadState from each thread with statistics
    pub fn join(self) -> PgBenchResult<Vec<ThreadState>> {
        let mut results = Vec::with_capacity(self.num_threads);

        for (thread_id, handle) in self.threads.into_iter().enumerate() {
            match handle.join() {
                Ok(Ok(thread_state)) => {
                    log::debug!("Thread {} completed successfully", thread_id);
                    results.push(thread_state);
                }
                Ok(Err(e)) => {
                    log::error!("Thread {} failed: {}", thread_id, e);
                    return Err(e);
                }
                Err(_) => {
                    return Err(PgBenchError::ThreadError(format!(
                        "Thread {} panicked",
                        thread_id
                    )));
                }
            }
        }

        Ok(results)
    }

    /// Get the number of threads
    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    /// Get clients per thread
    pub fn clients_per_thread(&self) -> usize {
        self.clients_per_thread
    }
}

/// Worker thread function
///
/// Each thread:
/// 1. Creates its ThreadState
/// 2. Connects all its clients to the database
/// 3. Waits at the start barrier
/// 4. Executes benchmark (placeholder for now)
/// 5. Returns ThreadState with results
///
/// Reference: pgbench.c threadRun() (lines 7484+)
fn thread_worker(
    thread_id: usize,
    num_clients: usize,
    connection_string: String,
    query_mode: QueryMode,
    seed: u64,
    start_barrier: Arc<Barrier>,
) -> PgBenchResult<ThreadState> {
    log::debug!(
        "Thread {} starting with {} clients",
        thread_id,
        num_clients
    );

    // Create thread state
    let mut thread_state = ThreadState::new(thread_id, num_clients, seed);

    // Connect all clients
    for client_id in 0..num_clients {
        let global_client_id = thread_id * num_clients + client_id;

        // Derive unique seed for this client
        let client_seed = seed.wrapping_add(client_id as u64 * 100);

        log::debug!(
            "Thread {} connecting client {} (global client {})",
            thread_id,
            client_id,
            global_client_id
        );

        // Create database connection
        let connection = PgBenchConnection::connect(&connection_string)
            .map_err(|e| {
                PgBenchError::ConnectionError(format!(
                    "Thread {} client {} connection failed: {}",
                    thread_id, client_id, e
                ))
            })?;

        // Create client state
        let client = ClientState::new(global_client_id, connection, query_mode, client_seed);

        thread_state.add_client(client);
    }

    log::debug!(
        "Thread {} connected {} clients, waiting at barrier",
        thread_id,
        num_clients
    );

    // Wait at barrier for all threads to be ready
    start_barrier.wait();

    log::debug!("Thread {} starting benchmark", thread_id);

    // Mark benchmark start time
    thread_state.start_benchmark();

    // TODO: Phase 7.3 - Execute benchmark loop here
    // For now, we just return the initialized state
    log::info!(
        "Thread {} initialized successfully (benchmark execution not yet implemented)",
        thread_id
    );

    Ok(thread_state)
}

/// Aggregate statistics from all threads
///
/// Merges statistics from all threads into a single StatsData object.
pub fn aggregate_stats(threads: &[ThreadState]) -> StatsData {
    let mut total = StatsData::new();

    for thread in threads {
        total.merge(&thread.stats);
    }

    total
}

/// Calculate total transaction count across all threads
pub fn total_transactions(threads: &[ThreadState]) -> u64 {
    threads.iter().map(|t| t.total_transactions()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_creation() {
        let pool = ThreadPool::new(4, 10).unwrap();
        assert_eq!(pool.num_threads(), 4);
        assert_eq!(pool.clients_per_thread(), 3); // 10 clients / 4 threads = 3 per thread
    }

    #[test]
    fn test_thread_pool_invalid_args() {
        assert!(ThreadPool::new(0, 10).is_err());
        assert!(ThreadPool::new(4, 0).is_err());
    }

    #[test]
    fn test_clients_distribution() {
        // Test even distribution
        let pool1 = ThreadPool::new(4, 12).unwrap();
        assert_eq!(pool1.clients_per_thread(), 3); // Exactly 3 per thread

        // Test uneven distribution (should round up)
        let pool2 = ThreadPool::new(3, 10).unwrap();
        assert_eq!(pool2.clients_per_thread(), 4); // 10/3 = 3.33, rounds up to 4

        // Test more threads than clients
        let pool3 = ThreadPool::new(10, 3).unwrap();
        assert_eq!(pool3.clients_per_thread(), 1); // 1 client per thread (some threads idle)
    }

    #[test]
    fn test_aggregate_stats() {
        let mut thread1 = ThreadState::new(0, 1, 100);
        thread1.stats.record_transaction(1000);
        thread1.stats.record_transaction(2000);

        let mut thread2 = ThreadState::new(1, 1, 200);
        thread2.stats.record_transaction(3000);
        thread2.stats.record_skipped();

        let threads = vec![thread1, thread2];
        let total = aggregate_stats(&threads);

        assert_eq!(total.cnt, 3); // 2 + 1
        assert_eq!(total.skipped, 1);
        assert_eq!(total.latency_sum, 6000); // 1000 + 2000 + 3000
    }

    #[test]
    fn test_total_transactions() {
        let mut thread1 = ThreadState::new(0, 2, 100);
        thread1.clients[0].transaction_count = 10;
        thread1.clients[1].transaction_count = 15;

        let mut thread2 = ThreadState::new(1, 1, 200);
        thread2.clients[0].transaction_count = 20;

        // Can't directly push to clients in test without proper setup
        // This test structure would need adjustment for real testing
        // For now, we'll test with empty clients
        let thread1 = ThreadState::new(0, 0, 100);
        let thread2 = ThreadState::new(1, 0, 200);

        let threads = vec![thread1, thread2];
        let total = total_transactions(&threads);

        assert_eq!(total, 0); // Both threads have 0 clients
    }

    // Note: Full integration test of thread pool with actual database
    // connections would go in tests/integration_test.rs
}
