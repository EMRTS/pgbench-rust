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
use stats::{print_latency_details, print_results, print_summary};
use worker::{aggregate_stats, BenchmarkConfig, ThreadPool};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments first (to check debug flag)
    let args = cli::Args::parse_args()?;

    // Initialize logging based on debug flag
    let log_level = if args.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    if !args.quiet {
        info!("pgbench-rust starting");
    }

    // Run in appropriate mode
    if args.initialize {
        run_initialize_mode(&args).await?;
    } else {
        run_benchmark_mode(&args).await?;
    }

    Ok(())
}

/// Run database initialization mode (-i flag) - async
async fn run_initialize_mode(args: &cli::Args) -> PgBenchResult<()> {
    info!("Initializing database...");

    // Connect to database (async)
    let mut conn = PgBenchConnection::connect(&args.connection).await?;

    // Run initialization (async)
    db::init::initialize_database(args, &mut conn).await?;

    info!("Database initialization complete");
    Ok(())
}

/// Run benchmark mode (default) - async
async fn run_benchmark_mode(args: &cli::Args) -> PgBenchResult<()> {
    info!("Running benchmark...");
    info!("Clients: {}, Threads: {}", args.clients, args.jobs);

    // Run VACUUM before benchmark unless --no-vacuum is specified
    if !args.no_vacuum {
        info!("Vacuuming tables before benchmark...");
        let mut conn = PgBenchConnection::connect(&args.connection).await?;

        // VACUUM cannot run inside a transaction block (async)
        conn.execute("VACUUM ANALYZE pgbench_branches", &[]).await?;
        conn.execute("VACUUM ANALYZE pgbench_tellers", &[]).await?;
        conn.execute("VACUUM ANALYZE pgbench_accounts", &[]).await?;
        conn.execute("VACUUM ANALYZE pgbench_history", &[]).await?;

        info!("VACUUM complete");
    }

    // Parse query mode
    let query_mode = QueryMode::from_str(&args.protocol).ok_or_else(|| {
        error::PgBenchError::InvalidArgument(format!("Invalid query mode: {}", args.protocol))
    })?;
    info!("Query mode: {}", query_mode.as_str());

    // Detect the actual scale factor from the database
    // This is critical for accurate benchmarking!
    let mut temp_conn = db::connection::PgBenchConnection::connect(&args.connection).await?;
    let actual_scale = db::detect_scale_factor(&mut temp_conn).await?;
    drop(temp_conn); // Close temporary connection

    info!("Detected scale factor: {} (from database)", actual_scale);

    // Create benchmark configuration with actual scale factor
    let mut config = BenchmarkConfig::new(actual_scale);

    if let Some(transactions) = args.transactions {
        config = config.with_transactions(transactions);
        info!("Transaction limit: {} per client", transactions);
    }

    if let Some(time) = args.time {
        config = config.with_time_limit(time);
        info!("Time limit: {} seconds", time);
    }

    // Get random seed (use provided seed or generate from time)
    let random_seed = args.random_seed.unwrap_or_else(|| {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    info!("Random seed: {}", random_seed);

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

    // Start all threads (barrier synchronization) - async
    pool.start().await;

    // Wait for all threads to complete - async
    let threads = pool.join().await?;

    // Calculate total benchmark duration
    let benchmark_duration = benchmark_start.elapsed();

    info!("Benchmark complete");

    // Aggregate statistics from all threads
    let total_stats = aggregate_stats(&threads);

    // Print results based on quiet mode
    println!();
    if args.quiet {
        // Quiet mode: just show transaction count and TPS
        print_summary(&total_stats, benchmark_duration);
    } else {
        // Normal mode: show full results
        let script_name = args
            .builtin
            .as_ref()
            .map(|b| format!("<builtin: {}>", b))
            .unwrap_or_else(|| "<builtin: TPC-B (sort of)>".to_string());

        print_results(
            &total_stats,
            args.clients,
            args.jobs,
            benchmark_duration,
            actual_scale,
            query_mode.as_str(),
            &script_name,
        );

        // Print latency details if enabled and requested
        if args.report_latencies && !total_stats.latencies.is_empty() {
            print_latency_details(&total_stats);
        }
    }

    Ok(())
}
