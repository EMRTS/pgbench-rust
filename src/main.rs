// pgbench-rust: A Rust port of PostgreSQL's pgbench benchmarking tool
//
// This is the main entry point for the pgbench command-line tool.

use anyhow::Result;
use log::info;
use std::time::Instant;

mod cli;
mod db;
mod error;
mod expr;
mod random;
mod script;
mod stats;
mod types;
mod utils;
mod worker;

use db::connection::PgBenchConnection;
use db::query::QueryMode;
use error::PgBenchResult;
use stats::{print_latency_details, print_results};
use worker::{aggregate_stats, BenchmarkConfig, ThreadPool};

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Parse command-line arguments
    let args = cli::Args::parse_args()?;

    info!("pgbench-rust starting");

    // Run in appropriate mode
    if args.initialize {
        run_initialize_mode(&args)?;
    } else {
        run_benchmark_mode(&args)?;
    }

    Ok(())
}

/// Run database initialization mode (-i flag)
fn run_initialize_mode(args: &cli::Args) -> PgBenchResult<()> {
    info!("Initializing database...");

    // Connect to database
    let mut conn = PgBenchConnection::connect(&args.connection)?;

    // Run initialization
    db::init::initialize_database(args, &mut conn)?;

    info!("Database initialization complete");
    Ok(())
}

/// Run benchmark mode (default)
fn run_benchmark_mode(args: &cli::Args) -> PgBenchResult<()> {
    info!("Running benchmark...");
    info!("Clients: {}, Threads: {}", args.clients, args.jobs);

    // Parse query mode
    let query_mode = QueryMode::from_str(&args.protocol).ok_or_else(|| {
        error::PgBenchError::InvalidArgument(format!("Invalid query mode: {}", args.protocol))
    })?;
    info!("Query mode: {}", query_mode.as_str());

    // Create benchmark configuration
    let mut config = BenchmarkConfig::new(args.scale as i64);

    if let Some(transactions) = args.transactions {
        config = config.with_transactions(transactions);
        info!("Transaction limit: {} per client", transactions);
    }

    if let Some(time) = args.time {
        config = config.with_time_limit(time);
        info!("Time limit: {} seconds", time);
    }

    // Get random seed (TODO: support --random-seed flag)
    let random_seed = 42; // Default seed for now

    // Create thread pool
    let pool = ThreadPool::new(args.jobs, args.clients)?;

    // Spawn worker threads
    let pool = pool.spawn_threads(
        args.connection.clone(),
        query_mode,
        random_seed,
        config.clone(),
    )?;

    // Record benchmark start time
    let benchmark_start = Instant::now();

    // Start all threads (barrier synchronization)
    pool.start();

    // Wait for all threads to complete
    let threads = pool.join()?;

    // Calculate total benchmark duration
    let benchmark_duration = benchmark_start.elapsed();

    info!("Benchmark complete");

    // Aggregate statistics from all threads
    let total_stats = aggregate_stats(&threads);

    // Print results
    println!();
    print_results(
        &total_stats,
        args.clients,
        args.jobs,
        benchmark_duration,
        args.scale as i64,
        query_mode.as_str(),
        "<builtin: TPC-B (sort of)>", // TODO: Get actual script name
    );

    // Print latency details if enabled
    // TODO: Add --report-latencies flag
    if !total_stats.latencies.is_empty() {
        print_latency_details(&total_stats);
    }

    Ok(())
}
