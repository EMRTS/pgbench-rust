//! Command-line interface for pgbench
//!
//! Provides command-line argument parsing using clap.
//! Aims for 100% compatibility with the original pgbench CLI.

use clap::{Parser, ArgGroup};
use anyhow::Result;

/// PostgreSQL benchmarking tool
#[derive(Parser, Debug)]
#[command(
    name = "pgbench",
    version,
    about = "PostgreSQL benchmarking tool (Rust port)",
    long_about = "A simple program for running benchmark tests on PostgreSQL.\n\
                  This is a Rust port of the original C implementation."
)]
#[command(group(
    ArgGroup::new("mode")
        .args(&["initialize", "benchmark"])
))]
pub struct Args {
    /// Initialize mode: create and populate tables
    #[arg(short = 'i', long)]
    pub initialize: bool,

    /// Benchmark mode (default)
    #[arg(long, hide = true)]
    benchmark: bool,

    /// Database connection string
    #[arg(value_name = "CONNECTION_STRING")]
    pub connection: String,

    /// Number of clients (concurrent database sessions)
    #[arg(short = 'c', long, default_value = "1")]
    pub clients: usize,

    /// Number of threads
    #[arg(short = 'j', long, default_value = "1")]
    pub jobs: usize,

    /// Number of transactions per client
    #[arg(short = 't', long)]
    pub transactions: Option<u64>,

    /// Duration of benchmark in seconds
    #[arg(short = 'T', long)]
    pub time: Option<u64>,

    /// Scaling factor (for initialization)
    #[arg(short = 's', long, default_value = "1")]
    pub scale: i32,

    /// Fillfactor percentage
    #[arg(short = 'F', long, default_value = "100")]
    pub fillfactor: i32,

    /// Use unlogged tables
    #[arg(long)]
    pub unlogged_tables: bool,

    /// Custom script file
    #[arg(short = 'f', long)]
    pub file: Option<Vec<String>>,

    /// Built-in script (simple-update, select-only, tpcb-like)
    #[arg(short = 'b', long)]
    pub builtin: Option<String>,

    /// Initialization steps (dtgvpf)
    #[arg(short = 'I', long, default_value = "dtgvp")]
    pub init_steps: String,

    /// Transaction rate limit (transactions per second)
    #[arg(short = 'R', long)]
    pub rate: Option<f64>,

    /// Latency limit in milliseconds
    #[arg(short = 'L', long)]
    pub latency_limit: Option<f64>,

    /// Number of database connections to establish
    #[arg(short = 'C', long)]
    pub connect: bool,

    /// Protocol to use (simple, extended, prepared)
    #[arg(short = 'M', long, default_value = "simple")]
    pub protocol: String,

    /// No vacuum before tests
    #[arg(short = 'n', long)]
    pub no_vacuum: bool,

    /// Log transactions
    #[arg(long)]
    pub log: bool,

    /// Report progress every N seconds
    #[arg(short = 'P', long)]
    pub progress: Option<u64>,

    /// Quiet mode
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Show debug output
    #[arg(short = 'd', long)]
    pub debug: bool,

    /// Random seed
    #[arg(long)]
    pub random_seed: Option<u64>,

    /// Sampling rate (0.0 to 1.0)
    #[arg(long)]
    pub sampling_rate: Option<f64>,

    /// Aggregate results per interval (seconds)
    #[arg(long)]
    pub aggregate_interval: Option<u64>,

    /// Report latencies
    #[arg(long)]
    pub report_latencies: bool,

    /// Number of partitions (for initialization)
    #[arg(long)]
    pub partitions: Option<i32>,

    /// Partition method (range, hash)
    #[arg(long)]
    pub partition_method: Option<String>,

    /// Tablespace for tables
    #[arg(long)]
    pub tablespace: Option<String>,

    /// Tablespace for indexes
    #[arg(long)]
    pub index_tablespace: Option<String>,
}

impl Args {
    /// Parse command-line arguments
    pub fn parse_args() -> Result<Self> {
        let args = Self::parse();

        // Validate arguments
        args.validate()?;

        Ok(args)
    }

    /// Validate command-line arguments
    fn validate(&self) -> Result<()> {
        // Check for conflicting options
        if self.transactions.is_some() && self.time.is_some() {
            anyhow::bail!("Cannot specify both -t and -T");
        }

        // Check thread count
        if self.jobs > self.clients {
            anyhow::bail!("Number of threads must not exceed number of clients");
        }

        // TEMPORARY LIMITATION: Warn about multiple clients per thread
        // This is due to using synchronous postgres operations which can deadlock
        // when multiple clients on the same thread compete for locks.
        // TODO: Remove this once we migrate to async operations (tokio-postgres)
        if self.jobs < self.clients && !self.initialize {
            log::warn!("WARNING: Running {} clients on {} threads", self.clients, self.jobs);
            log::warn!("         This configuration may deadlock due to synchronous database operations.");
            log::warn!("         If the program hangs, use -j {} (one thread per client) as a workaround.", self.clients);
            log::warn!("         Async operations will be implemented in v1.0.0 to fix this properly.");
        }

        // Check scale factor
        if self.scale < 1 {
            anyhow::bail!("Scale factor must be at least 1");
        }

        // Check fillfactor
        if self.fillfactor < 10 || self.fillfactor > 100 {
            anyhow::bail!("Fillfactor must be between 10 and 100");
        }

        // Check rate limit
        if let Some(rate) = self.rate {
            if rate <= 0.0 {
                anyhow::bail!("Rate limit must be positive");
            }
        }

        // Check sampling rate
        if let Some(sampling_rate) = self.sampling_rate {
            if !(0.0..=1.0).contains(&sampling_rate) {
                anyhow::bail!("Sampling rate must be between 0.0 and 1.0");
            }
        }

        Ok(())
    }

    /// Check if we're in initialize mode
    pub fn is_initialize_mode(&self) -> bool {
        self.initialize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_conflicting_duration() {
        let args = Args {
            initialize: false,
            benchmark: false,
            connection: "postgres://localhost/test".to_string(),
            clients: 1,
            jobs: 1,
            transactions: Some(100),
            time: Some(60),
            scale: 1,
            fillfactor: 100,
            unlogged_tables: false,
            file: None,
            builtin: None,
            init_steps: "dtgvp".to_string(),
            rate: None,
            latency_limit: None,
            connect: false,
            protocol: "simple".to_string(),
            no_vacuum: false,
            log: false,
            progress: None,
            quiet: false,
            debug: false,
            random_seed: None,
            sampling_rate: None,
            aggregate_interval: None,
            report_latencies: false,
            partitions: None,
            partition_method: None,
            tablespace: None,
            index_tablespace: None,
        };

        assert!(args.validate().is_err());
    }
}
