//! Worker thread implementation
//!
//! This module manages the thread pool for benchmark execution.
//! Reference: original-source/pgbench.c threadRun() (lines 7484+)

use crate::db::connection::PgBenchConnection;
use crate::db::query::QueryMode;
use crate::error::{PgBenchError, PgBenchResult};
use crate::script::{parse_script, BuiltinScript, ScriptExecutor};
use crate::worker::state::{ClientState, ConnectionState, StatsData, ThreadState};
use std::sync::{Arc, Barrier};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Benchmark configuration options
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Scale factor (number of scale units)
    pub scale: i64,

    /// Number of transactions per client (None = unlimited)
    pub transactions: Option<u64>,

    /// Time limit in seconds (None = unlimited)
    pub time_limit: Option<u64>,
}

impl BenchmarkConfig {
    /// Create a new benchmark configuration
    pub fn new(scale: i64) -> Self {
        Self {
            scale,
            transactions: None,
            time_limit: None,
        }
    }

    /// Set transaction limit per client
    pub fn with_transactions(mut self, transactions: u64) -> Self {
        self.transactions = Some(transactions);
        self
    }

    /// Set time limit in seconds
    pub fn with_time_limit(mut self, seconds: u64) -> Self {
        self.time_limit = Some(seconds);
        self
    }

    /// Get effective transaction limit (with default for testing)
    fn effective_transaction_limit(&self) -> Option<u64> {
        self.transactions.or_else(|| {
            if self.time_limit.is_none() {
                // Default to 10 transactions for testing if no limits specified
                Some(10)
            } else {
                None
            }
        })
    }
}

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
    /// * `config` - Benchmark configuration (scale, limits, etc.)
    ///
    /// # Returns
    /// Self with spawned threads
    pub fn spawn_threads(
        mut self,
        connection_string: String,
        query_mode: QueryMode,
        base_seed: u64,
        config: BenchmarkConfig,
    ) -> PgBenchResult<Self> {
        for thread_id in 0..self.num_threads {
            // Derive unique seed for this thread
            let thread_seed = base_seed.wrapping_add(thread_id as u64 * 1000);

            // Clone what we need to move into thread
            let barrier = Arc::clone(&self.start_barrier);
            let conn_str = connection_string.clone();
            let clients_per_thread = self.clients_per_thread;
            let bench_config = config.clone();

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
                        bench_config,
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
    config: BenchmarkConfig,
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
        let mut client = ClientState::new(global_client_id, connection, query_mode, client_seed);

        // Initialize standard pgbench variables
        client.initialize_standard_variables(config.scale, client_seed);

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

    // Parse the default TPC-B script (for now, hardcode to default script)
    // TODO: Support multiple scripts and script selection
    let builtin = BuiltinScript::get("tpcb-like")
        .ok_or_else(|| PgBenchError::ConfigError("TPC-B script not found".to_string()))?;

    let commands = parse_script(builtin.script)
        .map_err(|e| PgBenchError::ConfigError(format!("Failed to parse script: {}", e)))?;

    log::debug!("Thread {} parsed script with {} commands", thread_id, commands.len());

    // Main benchmark loop
    // Run until all clients are finished or limits reached
    let benchmark_start = Instant::now();
    let time_limit_duration = config.time_limit.map(|secs| Duration::from_secs(secs));

    while !thread_state.all_clients_finished() {
        // Check time limit
        if let Some(limit) = time_limit_duration {
            if benchmark_start.elapsed() >= limit {
                log::info!("Thread {} reached time limit ({} seconds)", thread_id, config.time_limit.unwrap());
                // Mark all clients as finished
                for client in &mut thread_state.clients {
                    if !matches!(client.state, ConnectionState::Finished | ConnectionState::Aborted) {
                        client.state = ConnectionState::Finished;
                    }
                }
                break;
            }
        }
        // Process each client
        for client_idx in 0..thread_state.num_clients() {
            let client = &mut thread_state.clients[client_idx];

            // Skip if client is already finished or aborted
            if matches!(client.state, ConnectionState::Finished | ConnectionState::Aborted) {
                continue;
            }

            // Check if client has reached transaction limit
            if let Some(limit) = config.effective_transaction_limit() {
                if client.transaction_count >= limit {
                    client.state = ConnectionState::Finished;
                    log::debug!("Client {} reached transaction limit ({})", client.id, limit);
                    continue;
                }
            }

            // Process client state machine
            match client.state {
                ConnectionState::ChooseScript => {
                    // Initialize variables for the script
                    // TODO: Initialize script-specific variables (scale, etc.)

                    // Start transaction
                    client.start_transaction();
                    client.script_index = 0;
                    client.command_index = 0;
                    client.state = ConnectionState::ExecuteCommand;

                    log::trace!("Client {} starting transaction {}", client.id, client.transaction_count + 1);
                }

                ConnectionState::ExecuteCommand => {
                    // Execute current command
                    if client.command_index >= commands.len() {
                        // All commands executed, end transaction
                        client.state = ConnectionState::EndTransaction;
                        continue;
                    }

                    let command = &commands[client.command_index];

                    match ScriptExecutor::execute(command, client) {
                        Ok(_) => {
                            // Command executed successfully, move to next
                            client.command_index += 1;
                        }
                        Err(e) => {
                            // Command failed
                            log::warn!("Client {} command failed: {}", client.id, e);

                            // TODO: Check if error is retryable (serialization, deadlock)
                            // For now, just abort the transaction
                            client.state = ConnectionState::Aborted;
                            thread_state.stats.record_failed(false, false);
                        }
                    }
                }

                ConnectionState::Sleep => {
                    // Check if sleep time has elapsed
                    if !client.should_sleep() {
                        client.sleep_until = None;
                        client.state = ConnectionState::ExecuteCommand;
                    }
                }

                ConnectionState::Throttle => {
                    // TODO: Implement throttling logic (--rate flag)
                    // For now, just move to next state
                    client.state = ConnectionState::ExecuteCommand;
                }

                ConnectionState::EndTransaction => {
                    // Transaction completed successfully
                    if let Some(latency_us) = client.end_transaction() {
                        thread_state.stats.record_transaction(latency_us);
                        client.transaction_count += 1;

                        log::trace!(
                            "Client {} completed transaction {} in {} μs",
                            client.id,
                            client.transaction_count,
                            latency_us
                        );
                    }

                    // Reset for next transaction
                    client.state = ConnectionState::ChooseScript;
                    client.tries = 0;
                }

                ConnectionState::Aborted => {
                    // Transaction aborted, reset and try again
                    log::debug!("Client {} transaction aborted, resetting", client.id);
                    client.state = ConnectionState::ChooseScript;
                    client.tries = 0;
                    client.txn_begin = None;
                }

                ConnectionState::Finished => {
                    // Client is done (shouldn't reach here due to continue above)
                    continue;
                }
            }
        }

        // Small sleep to avoid busy-waiting
        // TODO: Make this more efficient with proper event handling
        std::thread::sleep(Duration::from_micros(100));
    }

    log::info!(
        "Thread {} completed: {} total transactions",
        thread_id,
        thread_state.total_transactions()
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
        // Create threads with 0 clients (empty)
        let thread1 = ThreadState::new(0, 0, 100);
        let thread2 = ThreadState::new(1, 0, 200);

        let threads = vec![thread1, thread2];
        let total = total_transactions(&threads);

        // Both threads have 0 clients, so total should be 0
        assert_eq!(total, 0);

        // Note: Testing with actual clients would require proper ClientState setup
        // with real database connections, which is better suited for integration tests
    }

    #[test]
    fn test_benchmark_config_defaults() {
        let config = BenchmarkConfig::new(10);
        assert_eq!(config.scale, 10);
        assert_eq!(config.transactions, None);
        assert_eq!(config.time_limit, None);
    }

    #[test]
    fn test_benchmark_config_builder() {
        let config = BenchmarkConfig::new(5)
            .with_transactions(1000)
            .with_time_limit(60);

        assert_eq!(config.scale, 5);
        assert_eq!(config.transactions, Some(1000));
        assert_eq!(config.time_limit, Some(60));
    }

    #[test]
    fn test_effective_transaction_limit_explicit() {
        // When transactions is set, use it
        let config = BenchmarkConfig::new(1).with_transactions(500);
        assert_eq!(config.effective_transaction_limit(), Some(500));
    }

    #[test]
    fn test_effective_transaction_limit_default() {
        // When no limits set, default to 10 for testing
        let config = BenchmarkConfig::new(1);
        assert_eq!(config.effective_transaction_limit(), Some(10));
    }

    #[test]
    fn test_effective_transaction_limit_with_time_only() {
        // When only time limit set, no transaction limit
        let config = BenchmarkConfig::new(1).with_time_limit(60);
        assert_eq!(config.effective_transaction_limit(), None);
    }

    #[test]
    fn test_effective_transaction_limit_both_set() {
        // When both set, transactions takes precedence
        let config = BenchmarkConfig::new(1)
            .with_transactions(100)
            .with_time_limit(60);
        assert_eq!(config.effective_transaction_limit(), Some(100));
    }

    // Integration tests requiring database connection
    // These are marked with #[ignore] and run with: cargo test -- --ignored

    #[test]
    #[ignore]
    fn test_benchmark_execution_transaction_limit() {
        // TODO: Test that benchmark stops after N transactions
        // Requires: PostgreSQL database, connection string
        // Setup: Initialize pgbench tables
        // Run: Execute benchmark with transaction limit
        // Verify: Exactly N transactions executed per client
    }

    #[test]
    #[ignore]
    fn test_benchmark_execution_time_limit() {
        // TODO: Test that benchmark stops after N seconds
        // Requires: PostgreSQL database, connection string
        // Setup: Initialize pgbench tables
        // Run: Execute benchmark with time limit
        // Verify: Benchmark duration approximately N seconds
    }

    #[test]
    #[ignore]
    fn test_benchmark_with_variable_initialization() {
        // TODO: Test that script variables are properly initialized
        // Requires: PostgreSQL database, connection string
        // Setup: Initialize pgbench tables
        // Run: Execute TPC-B script
        // Verify: Variables :scale, :client_id, :random_seed are set
        // Verify: random() function uses correct ranges
    }

    // Note: Full integration test of thread pool with actual database
    // connections would go in tests/integration_test.rs
}
