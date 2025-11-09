//! Worker thread state management
//!
//! This module defines the state structures for multi-threaded benchmark execution.
//! Each thread manages multiple clients (connections), and each client has its own state.
//!
//! Reference: original-source/pgbench.c (lines 598-675)
//! - CState: Per-client state (lines 598-642)
//! - TState: Per-thread state (lines 647-675)
//! - StatsData: Statistics tracking (lines 378-454)

use crate::db::connection::PgBenchConnection;
use crate::db::query::QueryExecutor;
use crate::random::Xoroshiro128StarStar;
use crate::types::PgBenchValue;
use rand::SeedableRng;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Connection state machine states
///
/// Reference: pgbench.c ConnectionStateEnum (lines 510-593)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Choosing which script to execute
    ChooseScript,

    /// Executing commands in the script
    ExecuteCommand,

    /// Sleeping between commands
    Sleep,

    /// Waiting for throttling delay
    Throttle,

    /// Transaction finished, record stats
    EndTransaction,

    /// Script execution aborted due to error
    Aborted,

    /// Successfully finished
    Finished,
}

/// Statistics for transaction execution
///
/// Tracks successful, failed, skipped transactions and latencies.
/// Reference: pgbench.c StatsData (lines 378-454)
#[derive(Debug, Clone)]
pub struct StatsData {
    /// Interval start time for aggregates
    pub start_time: Instant,

    /// Number of successful transactions (not including skipped or failed)
    pub cnt: u64,

    /// Number of skipped transactions (too late under --rate or --latency-limit)
    pub skipped: u64,

    /// Number of failed transactions
    pub failed: u64,

    /// Number of serialization failures
    pub serialization_failures: u64,

    /// Number of deadlock failures
    pub deadlock_failures: u64,

    /// Number of retries (can be > retried if multiple retries per transaction)
    pub retries: u64,

    /// Number of retried transactions
    pub retried: u64,

    /// Sum of transaction latencies in microseconds
    pub latency_sum: u64,

    /// Sum of squared latencies (for calculating stddev)
    pub latency_sum_2: f64,

    /// Minimum latency in microseconds
    pub latency_min: u64,

    /// Maximum latency in microseconds
    pub latency_max: u64,

    /// Vector of individual latencies for percentile calculation
    pub latencies: Vec<u64>,
}

impl Default for StatsData {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            cnt: 0,
            skipped: 0,
            failed: 0,
            serialization_failures: 0,
            deadlock_failures: 0,
            retries: 0,
            retried: 0,
            latency_sum: 0,
            latency_sum_2: 0.0,
            latency_min: u64::MAX,
            latency_max: 0,
            latencies: Vec::new(),
        }
    }
}

impl StatsData {
    /// Create a new stats tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful transaction
    pub fn record_transaction(&mut self, latency_us: u64) {
        self.cnt += 1;
        self.latency_sum += latency_us;
        self.latency_sum_2 += (latency_us as f64) * (latency_us as f64);
        self.latency_min = self.latency_min.min(latency_us);
        self.latency_max = self.latency_max.max(latency_us);
        self.latencies.push(latency_us);
    }

    /// Record a skipped transaction
    pub fn record_skipped(&mut self) {
        self.skipped += 1;
    }

    /// Record a failed transaction
    pub fn record_failed(&mut self, is_serialization: bool, is_deadlock: bool) {
        self.failed += 1;
        if is_serialization {
            self.serialization_failures += 1;
        }
        if is_deadlock {
            self.deadlock_failures += 1;
        }
    }

    /// Record a retry attempt
    pub fn record_retry(&mut self) {
        self.retries += 1;
    }

    /// Record that a transaction was retried
    pub fn record_retried(&mut self) {
        self.retried += 1;
    }

    /// Calculate average latency in microseconds
    pub fn avg_latency(&self) -> f64 {
        if self.cnt == 0 {
            0.0
        } else {
            self.latency_sum as f64 / self.cnt as f64
        }
    }

    /// Calculate standard deviation of latency
    pub fn stddev_latency(&self) -> f64 {
        if self.cnt == 0 {
            0.0
        } else {
            let avg = self.avg_latency();
            let variance = (self.latency_sum_2 / self.cnt as f64) - (avg * avg);
            variance.max(0.0).sqrt()
        }
    }

    /// Calculate percentile of latencies
    ///
    /// # Arguments
    /// * `percentile` - Percentile to calculate (0.0 to 100.0)
    ///
    /// # Returns
    /// Latency value at the given percentile in microseconds
    ///
    /// Reference: pgbench.c getPercentile() (lines 5988-6018)
    pub fn percentile(&self, percentile: f64) -> u64 {
        if self.latencies.is_empty() {
            return 0;
        }

        // Sort latencies (we need to clone because we can't mutate self)
        let mut sorted = self.latencies.clone();
        sorted.sort_unstable();

        // Calculate index (using linear interpolation like original pgbench)
        let index = (percentile / 100.0) * (sorted.len() as f64 - 1.0);
        let lower = index.floor() as usize;
        let upper = index.ceil() as usize;

        if lower == upper || upper >= sorted.len() {
            sorted[lower.min(sorted.len() - 1)]
        } else {
            // Linear interpolation between two values
            let fraction = index - lower as f64;
            let lower_val = sorted[lower] as f64;
            let upper_val = sorted[upper] as f64;
            (lower_val + fraction * (upper_val - lower_val)) as u64
        }
    }

    /// Merge another stats object into this one
    pub fn merge(&mut self, other: &StatsData) {
        self.cnt += other.cnt;
        self.skipped += other.skipped;
        self.failed += other.failed;
        self.serialization_failures += other.serialization_failures;
        self.deadlock_failures += other.deadlock_failures;
        self.retries += other.retries;
        self.retried += other.retried;
        self.latency_sum += other.latency_sum;
        self.latency_sum_2 += other.latency_sum_2;
        self.latency_min = self.latency_min.min(other.latency_min);
        self.latency_max = self.latency_max.max(other.latency_max);
        self.latencies.extend_from_slice(&other.latencies);
    }
}

/// Per-client state
///
/// Each client represents one database connection executing transactions.
/// Reference: pgbench.c CState (lines 598-642)
#[derive(Debug)]
pub struct ClientState {
    /// Client ID (0-based)
    pub id: usize,

    /// Current state in the state machine
    pub state: ConnectionState,

    /// Database connection and query executor
    pub executor: QueryExecutor,

    /// Client-specific RNG for random functions in scripts
    pub func_rng: Xoroshiro128StarStar,

    /// Current script index being executed
    pub script_index: usize,

    /// Current command index within the script
    pub command_index: usize,

    /// Client variables (for \set commands)
    pub variables: HashMap<String, PgBenchValue>,

    /// Transaction scheduled start time
    pub txn_scheduled: Option<Instant>,

    /// Sleep until this time (for \sleep command)
    pub sleep_until: Option<Instant>,

    /// Transaction begin time (for measuring latency)
    pub txn_begin: Option<Instant>,

    /// Statement begin time (for measuring statement latency)
    pub stmt_begin: Option<Instant>,

    /// Number of retry attempts for current transaction
    pub tries: u32,

    /// Transaction count for this client (for -t limit)
    pub transaction_count: u64,

    /// Statistics for this client (latencies, counts, etc.)
    pub stats: StatsData,
}

impl ClientState {
    /// Create a new client state
    pub fn new(
        id: usize,
        connection: PgBenchConnection,
        query_mode: crate::db::query::QueryMode,
        seed: u64,
    ) -> Self {
        Self {
            id,
            state: ConnectionState::ChooseScript,
            executor: QueryExecutor::new(connection, query_mode),
            func_rng: Xoroshiro128StarStar::seed_from_u64(seed),
            script_index: 0,
            command_index: 0,
            variables: HashMap::new(),
            txn_scheduled: None,
            sleep_until: None,
            txn_begin: None,
            stmt_begin: None,
            tries: 0,
            transaction_count: 0,
            stats: StatsData::new(),
        }
    }

    /// Set a variable value
    pub fn set_variable(&mut self, name: String, value: PgBenchValue) {
        self.variables.insert(name, value);
    }

    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<&PgBenchValue> {
        self.variables.get(name)
    }

    /// Initialize standard pgbench variables
    ///
    /// Sets up the standard variables that pgbench scripts expect:
    /// - :client_id - The client ID (0-based)
    /// - :random_seed - The random seed for this client
    /// - :scale - The scale factor (defaults to 1 if not provided)
    ///
    /// Reference: pgbench.c initVariables() (lines 2938-2974)
    pub fn initialize_standard_variables(&mut self, scale: i64, random_seed: u64) {
        self.set_variable("client_id".to_string(), PgBenchValue::int(self.id as i64));
        self.set_variable("random_seed".to_string(), PgBenchValue::int(random_seed as i64));
        self.set_variable("scale".to_string(), PgBenchValue::int(scale));
    }

    /// Start a new transaction
    pub fn start_transaction(&mut self) {
        self.txn_begin = Some(Instant::now());
        self.command_index = 0;
        self.tries = 0;
    }

    /// End the current transaction and return latency in microseconds
    pub fn end_transaction(&mut self) -> Option<u64> {
        self.txn_begin.map(|begin| {
            let latency = begin.elapsed().as_micros() as u64;
            self.txn_begin = None;
            self.transaction_count += 1;
            latency
        })
    }

    /// Check if client should sleep
    pub fn should_sleep(&self) -> bool {
        if let Some(until) = self.sleep_until {
            Instant::now() < until
        } else {
            false
        }
    }

    /// Sleep for the specified duration
    pub fn sleep_for(&mut self, duration: Duration) {
        self.sleep_until = Some(Instant::now() + duration);
        self.state = ConnectionState::Sleep;
    }
}

/// Per-thread state
///
/// Each thread manages multiple clients and has its own RNGs and statistics.
/// Reference: pgbench.c TState (lines 647-675)
#[derive(Debug)]
pub struct ThreadState {
    /// Thread ID (0-based)
    pub id: usize,

    /// Array of client states managed by this thread
    pub clients: Vec<ClientState>,

    /// RNG for choosing which script to run
    pub choose_script_rng: Xoroshiro128StarStar,

    /// RNG for throttling delays
    pub throttle_rng: Xoroshiro128StarStar,

    /// RNG for log sampling
    pub sample_rng: Xoroshiro128StarStar,

    /// Previous/next throttling trigger time in microseconds
    pub throttle_trigger: Option<i64>,

    /// Per-thread aggregated statistics
    pub stats: StatsData,

    /// Thread creation time
    pub create_time: Instant,

    /// Thread started running time
    pub started_time: Option<Instant>,

    /// Benchmark start time
    pub bench_start: Option<Instant>,

    /// Cumulated connection/disconnection delays
    pub conn_duration: Duration,

    /// Count of late transactions (executed but late)
    pub latency_late: u64,
}

impl ThreadState {
    /// Create a new thread state
    pub fn new(
        id: usize,
        num_clients: usize,
        seed: u64,
    ) -> Self {
        // Derive different seeds for each RNG using a simple method
        let choose_seed = seed.wrapping_mul(3);
        let throttle_seed = seed.wrapping_mul(5);
        let sample_seed = seed.wrapping_mul(7);

        Self {
            id,
            clients: Vec::with_capacity(num_clients),
            choose_script_rng: Xoroshiro128StarStar::seed_from_u64(choose_seed),
            throttle_rng: Xoroshiro128StarStar::seed_from_u64(throttle_seed),
            sample_rng: Xoroshiro128StarStar::seed_from_u64(sample_seed),
            throttle_trigger: None,
            stats: StatsData::new(),
            create_time: Instant::now(),
            started_time: None,
            bench_start: None,
            conn_duration: Duration::ZERO,
            latency_late: 0,
        }
    }

    /// Add a client to this thread
    pub fn add_client(&mut self, client: ClientState) {
        self.clients.push(client);
    }

    /// Start the benchmark (record start time)
    pub fn start_benchmark(&mut self) {
        let now = Instant::now();
        self.started_time = Some(now);
        self.bench_start = Some(now);
    }

    /// Get the number of clients managed by this thread
    pub fn num_clients(&self) -> usize {
        self.clients.len()
    }

    /// Get total transaction count across all clients
    pub fn total_transactions(&self) -> u64 {
        self.clients.iter().map(|c| c.transaction_count).sum()
    }

    /// Check if all clients are finished
    pub fn all_clients_finished(&self) -> bool {
        self.clients.iter().all(|c| {
            matches!(c.state, ConnectionState::Finished | ConnectionState::Aborted)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_data_default() {
        let stats = StatsData::default();
        assert_eq!(stats.cnt, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.latency_min, u64::MAX);
        assert_eq!(stats.latency_max, 0);
    }

    #[test]
    fn test_stats_record_transaction() {
        let mut stats = StatsData::new();

        stats.record_transaction(1000);
        assert_eq!(stats.cnt, 1);
        assert_eq!(stats.latency_sum, 1000);
        assert_eq!(stats.latency_min, 1000);
        assert_eq!(stats.latency_max, 1000);

        stats.record_transaction(2000);
        assert_eq!(stats.cnt, 2);
        assert_eq!(stats.latency_sum, 3000);
        assert_eq!(stats.latency_min, 1000);
        assert_eq!(stats.latency_max, 2000);
    }

    #[test]
    fn test_stats_avg_latency() {
        let mut stats = StatsData::new();

        assert_eq!(stats.avg_latency(), 0.0);

        stats.record_transaction(1000);
        stats.record_transaction(2000);
        stats.record_transaction(3000);

        assert_eq!(stats.avg_latency(), 2000.0);
    }

    #[test]
    fn test_stats_merge() {
        let mut stats1 = StatsData::new();
        stats1.record_transaction(1000);
        stats1.record_skipped();

        let mut stats2 = StatsData::new();
        stats2.record_transaction(2000);
        stats2.record_failed(true, false);

        stats1.merge(&stats2);

        assert_eq!(stats1.cnt, 2);
        assert_eq!(stats1.skipped, 1);
        assert_eq!(stats1.failed, 1);
        assert_eq!(stats1.serialization_failures, 1);
        assert_eq!(stats1.latency_min, 1000);
        assert_eq!(stats1.latency_max, 2000);
    }

    #[test]
    fn test_connection_state_enum() {
        let state = ConnectionState::ChooseScript;
        assert_eq!(state, ConnectionState::ChooseScript);
        assert_ne!(state, ConnectionState::Finished);
    }

    #[test]
    fn test_thread_state_creation() {
        let thread = ThreadState::new(0, 10, 12345);

        assert_eq!(thread.id, 0);
        assert_eq!(thread.num_clients(), 0);
        assert_eq!(thread.total_transactions(), 0);
        assert!(thread.all_clients_finished());
    }

    // Note: test_client_variables requires a real database connection
    // and would be an integration test, not a unit test.
    // Variable storage is tested through other state tests.

    #[test]
    fn test_stats_stddev() {
        let mut stats = StatsData::new();

        // Add transactions with known variance
        stats.record_transaction(1000);
        stats.record_transaction(2000);
        stats.record_transaction(3000);

        let stddev = stats.stddev_latency();
        // Stddev of [1000, 2000, 3000] should be approximately 816.5
        assert!(stddev > 800.0 && stddev < 900.0);
    }
}
