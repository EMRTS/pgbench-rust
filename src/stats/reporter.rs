//! Statistics reporter

use crate::types::TransactionStats;
use crate::error::PgBenchResult;
use crate::utils;

/// Report final benchmark results
pub fn report_results(stats: &TransactionStats, duration_us: u64) -> PgBenchResult<()> {
    println!("\n=== Benchmark Results ===");
    println!("Total transactions: {}", stats.count);
    println!("Duration: {}", utils::format_duration(duration_us));

    if stats.count > 0 {
        let tps = stats.count as f64 / (duration_us as f64 / 1_000_000.0);
        println!("TPS: {:.2}", tps);

        let avg_latency = stats.total_time as f64 / stats.count as f64;
        println!("Average latency: {}", utils::format_duration(avg_latency as u64));
        println!("Min latency: {}", utils::format_duration(stats.min_latency));
        println!("Max latency: {}", utils::format_duration(stats.max_latency));
    }

    Ok(())
}

/// Report progress during benchmark
pub fn report_progress(_stats: &TransactionStats, _elapsed_us: u64) {
    // TODO: Implement progress reporting
}

// TODO: Implement detailed latency reporting
// TODO: Implement percentile reporting
// TODO: Implement log file generation
// TODO: Match output format of original pgbench
