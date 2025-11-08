//! Statistics reporter
//!
//! Formats and displays benchmark results matching the original pgbench output format.
//!
//! Reference: original-source/pgbench.c printResults() (lines 7048-7201)

use crate::worker::state::StatsData;
use std::time::Duration;

/// Report final benchmark results
///
/// Formats output to match original pgbench format:
/// ```text
/// transaction type: <TPC-B (sort of)>
/// scaling factor: 1
/// query mode: simple
/// number of clients: 1
/// number of threads: 1
/// duration: 10 s
/// number of transactions actually processed: 1234
/// latency average = 8.100 ms
/// latency stddev = 1.234 ms
/// tps = 123.456789 (including connections establishing)
/// ```
///
/// Reference: pgbench.c printResults() (lines 7048-7201)
pub fn print_results(
    stats: &StatsData,
    num_clients: usize,
    num_threads: usize,
    duration: Duration,
    scale: i64,
    query_mode: &str,
    script_name: &str,
) {
    let duration_secs = duration.as_secs_f64();

    println!("transaction type: {}", script_name);
    println!("scaling factor: {}", scale);
    println!("query mode: {}", query_mode);
    println!("number of clients: {}", num_clients);
    println!("number of threads: {}", num_threads);
    println!("duration: {} s", duration_secs as u64);
    println!("number of transactions actually processed: {}", stats.cnt);

    if stats.cnt > 0 {
        // Latency in milliseconds
        let avg_latency_ms = stats.avg_latency() / 1000.0;
        let stddev_latency_ms = stats.stddev_latency() / 1000.0;

        println!("latency average = {:.3} ms", avg_latency_ms);
        println!("latency stddev = {:.3} ms", stddev_latency_ms);

        // TPS calculation
        let tps = stats.cnt as f64 / duration_secs;
        println!("tps = {:.6} (including connections establishing)", tps);
    }

    // Additional statistics if there were failures
    if stats.failed > 0 || stats.skipped > 0 {
        println!("number of transactions skipped: {}", stats.skipped);
        println!("number of transactions failed: {}", stats.failed);

        if stats.serialization_failures > 0 {
            println!("  - serialization failures: {}", stats.serialization_failures);
        }
        if stats.deadlock_failures > 0 {
            println!("  - deadlock failures: {}", stats.deadlock_failures);
        }
    }

    if stats.retried > 0 {
        println!("number of transactions retried: {}", stats.retried);
        println!("total number of retries: {}", stats.retries);
    }
}

/// Print detailed latency statistics including percentiles
///
/// Reference: pgbench.c printResults() percentile section
pub fn print_latency_details(stats: &StatsData) {
    if stats.latencies.is_empty() {
        return;
    }

    println!("\nlatency statistics:");
    println!("         min: {:.3} ms", stats.latency_min as f64 / 1000.0);
    println!("         max: {:.3} ms", stats.latency_max as f64 / 1000.0);

    // Print percentiles (P50, P90, P95, P99)
    let percentiles = vec![50.0, 90.0, 95.0, 99.0];
    for p in percentiles {
        let value_us = stats.percentile(p);
        let value_ms = value_us as f64 / 1000.0;
        println!("         p{}: {:.3} ms", p as u64, value_ms);
    }
}

/// Report progress during benchmark (for -P flag)
///
/// Outputs a progress line every N seconds showing:
/// - Interval number
/// - TPS for this interval
/// - Average latency for this interval
///
/// Example: `progress: 5.0 s, 1234.5 tps, lat 8.123 ms stddev 1.234`
///
/// Reference: pgbench.c printProgressReport() (lines 5832-5898)
pub fn print_progress(
    interval_num: u64,
    interval_secs: f64,
    transactions: u64,
    avg_latency_us: f64,
    stddev_latency_us: f64,
) {
    let tps = transactions as f64 / interval_secs;
    let avg_latency_ms = avg_latency_us / 1000.0;
    let stddev_latency_ms = stddev_latency_us / 1000.0;

    println!(
        "progress: {:.1} s, {:.1} tps, lat {:.3} ms stddev {:.3}",
        interval_num as f64 * interval_secs,
        tps,
        avg_latency_ms,
        stddev_latency_ms
    );
}

/// Print a simple summary (for quiet mode)
pub fn print_summary(stats: &StatsData, duration: Duration) {
    let duration_secs = duration.as_secs_f64();
    let tps = if duration_secs > 0.0 {
        stats.cnt as f64 / duration_secs
    } else {
        0.0
    };

    println!("{} transactions ({:.2} tps)", stats.cnt, tps);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_results_output() {
        // Create sample stats
        let mut stats = StatsData::new();
        stats.record_transaction(5000);  // 5ms
        stats.record_transaction(10000); // 10ms
        stats.record_transaction(15000); // 15ms

        // Test that it doesn't panic (actual output would go to stdout)
        print_results(
            &stats,
            1,
            1,
            Duration::from_secs(10),
            1,
            "simple",
            "<builtin: TPC-B (sort of)>",
        );

        // Verify calculations
        assert_eq!(stats.cnt, 3);
        assert_eq!(stats.avg_latency(), 10000.0);
    }

    #[test]
    fn test_print_summary() {
        let mut stats = StatsData::new();
        stats.record_transaction(1000);
        stats.record_transaction(2000);

        print_summary(&stats, Duration::from_secs(2));
        // Output: "2 transactions (1.00 tps)"
    }

    #[test]
    fn test_print_progress() {
        print_progress(1, 5.0, 100, 5000.0, 500.0);
        // Output: "progress: 5.0 s, 20.0 tps, lat 5.000 ms stddev 0.500"
    }
}
