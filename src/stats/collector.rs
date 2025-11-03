//! Statistics collector

use crate::types::TransactionStats;

/// Collects and aggregates statistics from worker threads
pub struct StatsCollector {
    stats: TransactionStats,
}

impl StatsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            stats: Default::default(),
        }
    }

    /// Record a transaction
    pub fn record_transaction(&mut self, latency_us: u64) {
        self.stats.count += 1;
        self.stats.total_time += latency_us;

        let latency_f = latency_us as f64;
        self.stats.sum_squared += latency_f * latency_f;

        if self.stats.count == 1 || latency_us < self.stats.min_latency {
            self.stats.min_latency = latency_us;
        }

        if latency_us > self.stats.max_latency {
            self.stats.max_latency = latency_us;
        }
    }

    /// Get the collected statistics
    pub fn stats(&self) -> &TransactionStats {
        &self.stats
    }

    /// Calculate average latency
    pub fn average_latency(&self) -> f64 {
        if self.stats.count == 0 {
            0.0
        } else {
            self.stats.total_time as f64 / self.stats.count as f64
        }
    }

    /// Calculate standard deviation of latency
    pub fn stddev_latency(&self) -> f64 {
        if self.stats.count < 2 {
            0.0
        } else {
            let avg = self.average_latency();
            let variance = self.stats.sum_squared / self.stats.count as f64 - avg * avg;
            variance.sqrt()
        }
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implement percentile calculation
// TODO: Implement histogram support
// TODO: Implement per-transaction-type statistics
