//! Database initialization for benchmarking
//!
//! Creates and populates the pgbench tables (pgbench_accounts, pgbench_branches,
//! pgbench_tellers, pgbench_history) according to the scale factor.
//!
//! Reference: original-source/pgbench.c lines 4775-5250

use crate::cli::Args;
use crate::db::connection::PgBenchConnection;
use crate::error::{PgBenchError, PgBenchResult};
use std::io::{self, Write};
use std::time::Instant;

// Constants from original pgbench
// Reference: pgbench.c lines 244-247
const BRANCHES_PER_SCALE: i64 = 1;
const TELLERS_PER_SCALE: i64 = 10;
const ACCOUNTS_PER_SCALE: i64 = 100_000;

/// Scale threshold where we switch from int to bigint for account IDs
/// Reference: pgbench.c line 256
const SCALE_32BIT_THRESHOLD: i32 = 20000;

/// Initialize the database with benchmark tables
///
/// This is the main entry point for the `pgbench -i` mode.
/// It handles all initialization steps according to the args.
///
/// Reference: pgbench.c runInit() function
pub fn initialize_database(args: &Args, conn: &mut PgBenchConnection) -> PgBenchResult<()> {
    eprintln!("pgbench: initializing database...");

    // Parse init steps (default: "dtgvp")
    let init_steps = parse_init_steps(&args.init_steps)?;

    // Execute each init step in order
    for step in init_steps {
        match step {
            InitStep::DropTables => {
                drop_tables(conn)?;
            }
            InitStep::CreateTables => {
                create_tables(conn, args)?;
            }
            InitStep::GenerateDataClientSide => {
                generate_data_client_side(conn, args)?;
            }
            InitStep::GenerateDataServerSide => {
                generate_data_server_side(conn, args)?;
            }
            InitStep::Vacuum => {
                vacuum_tables(conn)?;
            }
            InitStep::CreatePrimaryKeys => {
                create_primary_keys(conn)?;
            }
            InitStep::CreateForeignKeys => {
                create_foreign_keys(conn)?;
            }
        }
    }

    eprintln!("pgbench: initialization complete");
    Ok(())
}

/// Initialization steps
/// Reference: pgbench.c initSteps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitStep {
    /// Drop tables (d)
    DropTables,
    /// Create tables (t)
    CreateTables,
    /// Generate data client-side (g)
    GenerateDataClientSide,
    /// Generate data server-side (G)
    GenerateDataServerSide,
    /// Vacuum (v)
    Vacuum,
    /// Create primary keys (p)
    CreatePrimaryKeys,
    /// Create foreign keys (f)
    CreateForeignKeys,
}

/// Parse init steps from string (e.g., "dtgvp")
fn parse_init_steps(steps_str: &str) -> PgBenchResult<Vec<InitStep>> {
    let mut steps = Vec::new();

    for c in steps_str.chars() {
        let step = match c {
            'd' => InitStep::DropTables,
            't' => InitStep::CreateTables,
            'g' => InitStep::GenerateDataClientSide,
            'G' => InitStep::GenerateDataServerSide,
            'v' => InitStep::Vacuum,
            'p' => InitStep::CreatePrimaryKeys,
            'f' => InitStep::CreateForeignKeys,
            _ => {
                return Err(PgBenchError::InvalidArgument(
                    format!("invalid init step '{}', valid steps are: d, t, g, G, v, p, f", c)
                ));
            }
        };
        steps.push(step);
    }

    Ok(steps)
}

/// Drop all pgbench tables
///
/// Reference: pgbench.c initDropTables() line 4775
fn drop_tables(conn: &mut PgBenchConnection) -> PgBenchResult<()> {
    eprintln!("dropping old tables...");

    // Drop all tables in one command (handles foreign key dependencies)
    conn.execute(
        "DROP TABLE IF EXISTS pgbench_accounts, pgbench_branches, pgbench_history, pgbench_tellers",
        &[],
    )?;

    Ok(())
}

/// Create all pgbench tables
///
/// Reference: pgbench.c initCreateTables() line 4866
fn create_tables(conn: &mut PgBenchConnection, args: &Args) -> PgBenchResult<()> {
    eprintln!("creating tables...");

    let scale = args.scale;
    let use_bigint = scale >= SCALE_32BIT_THRESHOLD;
    let unlogged = args.unlogged_tables;
    let fillfactor = args.fillfactor;
    let tablespace = args.tablespace.as_deref();

    // Table definitions
    // Reference: pgbench.c DDLs array line 4886
    let tables = [
        TableDef {
            name: "pgbench_history",
            columns_32bit: "tid int, bid int, aid int, delta int, mtime timestamp, filler char(22)",
            columns_64bit: "tid int, bid int, aid bigint, delta int, mtime timestamp, filler char(22)",
            use_fillfactor: false,
        },
        TableDef {
            name: "pgbench_tellers",
            columns_32bit: "tid int not null, bid int, tbalance int, filler char(84)",
            columns_64bit: "tid int not null, bid int, tbalance int, filler char(84)",
            use_fillfactor: true,
        },
        TableDef {
            name: "pgbench_accounts",
            columns_32bit: "aid int not null, bid int, abalance int, filler char(84)",
            columns_64bit: "aid bigint not null, bid int, abalance int, filler char(84)",
            use_fillfactor: true,
        },
        TableDef {
            name: "pgbench_branches",
            columns_32bit: "bid int not null, bbalance int, filler char(88)",
            columns_64bit: "bid int not null, bbalance int, filler char(88)",
            use_fillfactor: true,
        },
    ];

    for table in &tables {
        let mut sql = String::new();

        // CREATE [UNLOGGED] TABLE name
        sql.push_str("CREATE");
        if unlogged && args.partitions.is_none() {
            sql.push_str(" UNLOGGED");
        }
        sql.push_str(" TABLE ");
        sql.push_str(table.name);
        sql.push('(');

        // Columns
        if use_bigint {
            sql.push_str(table.columns_64bit);
        } else {
            sql.push_str(table.columns_32bit);
        }
        sql.push(')');

        // Partitioning (only for pgbench_accounts)
        if args.partitions.is_some() && table.name == "pgbench_accounts" {
            sql.push_str(" PARTITION BY ");
            let method = args.partition_method.as_deref().unwrap_or("hash");
            if method == "range" {
                sql.push_str("RANGE");
            } else if method == "hash" {
                sql.push_str("HASH");
            } else {
                return Err(PgBenchError::InvalidArgument(format!("Unknown partition method: {}", method)));
            }
            sql.push_str(" (aid)");
        } else if table.use_fillfactor {
            // Fillfactor (only for non-partitioned tables)
            sql.push_str(&format!(" WITH (fillfactor={})", fillfactor));
        }

        // Tablespace
        if let Some(ts) = tablespace {
            sql.push_str(&format!(" TABLESPACE {}", ts));
        }

        conn.execute(&sql, &[])?;
    }

    // Create partitions if requested
    if args.partitions.is_some() {
        create_partitions(conn, args)?;
    }

    Ok(())
}

/// Table definition helper
struct TableDef {
    name: &'static str,
    columns_32bit: &'static str,
    columns_64bit: &'static str,
    use_fillfactor: bool,
}

/// Create partitions for pgbench_accounts table
///
/// Reference: pgbench.c createPartitions() line 4797
fn create_partitions(conn: &mut PgBenchConnection, args: &Args) -> PgBenchResult<()> {
    let num_partitions = args.partitions.unwrap(); // Safe: caller checks is_some()
    let scale = args.scale;
    let fillfactor = args.fillfactor;
    let unlogged = args.unlogged_tables;
    let is_range = args.partition_method.as_deref() == Some("range");

    eprintln!("creating {} partitions...", num_partitions);

    let total_accounts = ACCOUNTS_PER_SCALE * scale as i64;
    let part_size = (total_accounts + num_partitions as i64 - 1) / num_partitions as i64;

    for p in 1..=num_partitions {
        let mut sql = String::new();

        sql.push_str("CREATE");
        if unlogged {
            sql.push_str(" UNLOGGED");
        }
        sql.push_str(&format!(" TABLE pgbench_accounts_{}", p));
        sql.push_str(" PARTITION OF pgbench_accounts");

        if is_range {
            sql.push_str(" FOR VALUES FROM (");
            if p == 1 {
                sql.push_str("MINVALUE");
            } else {
                sql.push_str(&format!("{}", (p as i64 - 1) * part_size + 1));
            }
            sql.push_str(") TO (");
            if p < num_partitions {
                sql.push_str(&format!("{}", p as i64 * part_size + 1));
            } else {
                sql.push_str("MAXVALUE");
            }
            sql.push(')');
        } else {
            // Hash partitioning
            sql.push_str(&format!(
                " FOR VALUES WITH (MODULUS {}, REMAINDER {})",
                num_partitions,
                p - 1
            ));
        }

        // Fillfactor for partitions
        sql.push_str(&format!(" WITH (fillfactor={})", fillfactor));

        conn.execute(&sql, &[])?;
    }

    Ok(())
}

/// Generate data client-side using COPY
///
/// Reference: pgbench.c initGenerateDataClientSide() line 5123
fn generate_data_client_side(conn: &mut PgBenchConnection, args: &Args) -> PgBenchResult<()> {
    eprintln!("generating data (client-side)...");

    let scale = args.scale as i64;

    // Populate branches
    populate_table(
        conn,
        "pgbench_branches",
        BRANCHES_PER_SCALE * scale,
        generate_branch_row,
        args.quiet,
    )?;

    // Populate tellers
    populate_table(
        conn,
        "pgbench_tellers",
        TELLERS_PER_SCALE * scale,
        generate_teller_row,
        args.quiet,
    )?;

    // Populate accounts (largest table)
    populate_table(
        conn,
        "pgbench_accounts",
        ACCOUNTS_PER_SCALE * scale,
        generate_account_row,
        args.quiet,
    )?;

    Ok(())
}

/// Generate data server-side using INSERT ... SELECT
///
/// Reference: pgbench.c initGenerateDataServerSide() line 5155
fn generate_data_server_side(conn: &mut PgBenchConnection, args: &Args) -> PgBenchResult<()> {
    eprintln!("generating data (server-side)...");

    let scale = args.scale as i64;

    // Branches
    let sql = format!(
        "INSERT INTO pgbench_branches(bid, bbalance) \
         SELECT bid, 0 FROM generate_series(1, {}) AS bid",
        BRANCHES_PER_SCALE * scale
    );
    conn.execute(&sql, &[])?;

    // Tellers
    let sql = format!(
        "INSERT INTO pgbench_tellers(tid, bid, tbalance) \
         SELECT tid, (tid - 1) / {} + 1, 0 FROM generate_series(1, {}) AS tid",
        TELLERS_PER_SCALE,
        TELLERS_PER_SCALE * scale
    );
    conn.execute(&sql, &[])?;

    // Accounts (with blank filler)
    let sql = format!(
        "INSERT INTO pgbench_accounts(aid, bid, abalance, filler) \
         SELECT aid, (aid - 1) / {} + 1, 0, '' FROM generate_series(1, {}) AS aid",
        ACCOUNTS_PER_SCALE,
        ACCOUNTS_PER_SCALE * scale
    );
    conn.execute(&sql, &[])?;

    Ok(())
}

/// Populate a table using COPY protocol
///
/// Reference: pgbench.c initPopulateTable() line 4998
fn populate_table<F>(
    conn: &mut PgBenchConnection,
    table_name: &str,
    row_count: i64,
    generate_row: F,
    quiet: bool,
) -> PgBenchResult<()>
where
    F: Fn(i64) -> String,
{
    eprintln!("generating {} rows for {}...", row_count, table_name);

    let start = Instant::now();
    let mut writer = conn.copy_in(&format!("COPY {} FROM STDIN", table_name))?;

    for k in 0..row_count {
        let row_data = generate_row(k);
        writer.write_all(row_data.as_bytes())?;

        // Progress reporting (every 100k rows)
        if !quiet && (k + 1) % 100_000 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let remaining = ((row_count - k - 1) as f64) * elapsed / (k + 1) as f64;
            eprint!(
                "\r{} of {} tuples ({}%) of {} done (elapsed {:.2}s, remaining {:.2}s)",
                k + 1,
                row_count,
                ((k + 1) * 100) / row_count,
                table_name,
                elapsed,
                remaining
            );
            io::stderr().flush()?;
        }
    }

    writer.finish()?;

    if !quiet {
        eprintln!(); // New line after progress
    }

    Ok(())
}

/// Generate a branch row for COPY
/// Reference: pgbench.c initBranch() line 4971
fn generate_branch_row(k: i64) -> String {
    // Format: bid \t bbalance \t filler (NULL)
    format!("{}\t0\t\\N\n", k + 1)
}

/// Generate a teller row for COPY
/// Reference: pgbench.c initTeller() line 4980
fn generate_teller_row(k: i64) -> String {
    // Format: tid \t bid \t tbalance \t filler (NULL)
    let tid = k + 1;
    let bid = k / TELLERS_PER_SCALE + 1;
    format!("{}\t{}\t0\t\\N\n", tid, bid)
}

/// Generate an account row for COPY
/// Reference: pgbench.c initAccount() line 4989
fn generate_account_row(k: i64) -> String {
    // Format: aid \t bid \t abalance \t filler (blank padded)
    let aid = k + 1;
    let bid = k / ACCOUNTS_PER_SCALE + 1;
    format!("{}\t{}\t0\t\n", aid, bid)
}

/// Run VACUUM on all tables
///
/// Reference: pgbench.c initVacuum() line 5200
fn vacuum_tables(conn: &mut PgBenchConnection) -> PgBenchResult<()> {
    eprintln!("vacuuming...");

    // VACUUM cannot run inside a transaction block
    conn.execute("VACUUM ANALYZE pgbench_branches", &[])?;
    conn.execute("VACUUM ANALYZE pgbench_tellers", &[])?;
    conn.execute("VACUUM ANALYZE pgbench_accounts", &[])?;
    conn.execute("VACUUM ANALYZE pgbench_history", &[])?;

    Ok(())
}

/// Create primary keys on all tables
///
/// Reference: pgbench.c initCreatePKeys() line 5217
fn create_primary_keys(conn: &mut PgBenchConnection) -> PgBenchResult<()> {
    eprintln!("creating primary keys...");

    conn.execute(
        "ALTER TABLE pgbench_branches ADD PRIMARY KEY (bid)",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE pgbench_tellers ADD PRIMARY KEY (tid)",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE pgbench_accounts ADD PRIMARY KEY (aid)",
        &[],
    )?;

    Ok(())
}

/// Create foreign keys between tables
///
/// Reference: pgbench.c initCreateFKeys() line 5236
fn create_foreign_keys(conn: &mut PgBenchConnection) -> PgBenchResult<()> {
    eprintln!("creating foreign keys...");

    conn.execute(
        "ALTER TABLE pgbench_tellers ADD CONSTRAINT pgbench_tellers_bid_fkey \
         FOREIGN KEY (bid) REFERENCES pgbench_branches",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE pgbench_accounts ADD CONSTRAINT pgbench_accounts_bid_fkey \
         FOREIGN KEY (bid) REFERENCES pgbench_branches",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE pgbench_history ADD CONSTRAINT pgbench_history_bid_fkey \
         FOREIGN KEY (bid) REFERENCES pgbench_branches",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE pgbench_history ADD CONSTRAINT pgbench_history_tid_fkey \
         FOREIGN KEY (tid) REFERENCES pgbench_tellers",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE pgbench_history ADD CONSTRAINT pgbench_history_aid_fkey \
         FOREIGN KEY (aid) REFERENCES pgbench_accounts",
        &[],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_steps_default() {
        let steps = parse_init_steps("dtgvp").unwrap();
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0], InitStep::DropTables);
        assert_eq!(steps[1], InitStep::CreateTables);
        assert_eq!(steps[2], InitStep::GenerateDataClientSide);
        assert_eq!(steps[3], InitStep::Vacuum);
        assert_eq!(steps[4], InitStep::CreatePrimaryKeys);
    }

    #[test]
    fn test_parse_init_steps_with_foreign_keys() {
        let steps = parse_init_steps("dtgvpf").unwrap();
        assert_eq!(steps.len(), 6);
        assert_eq!(steps[5], InitStep::CreateForeignKeys);
    }

    #[test]
    fn test_parse_init_steps_server_side() {
        let steps = parse_init_steps("dtGvp").unwrap();
        assert_eq!(steps[2], InitStep::GenerateDataServerSide);
    }

    #[test]
    fn test_parse_init_steps_invalid() {
        let result = parse_init_steps("dtx");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_branch_row() {
        let row = generate_branch_row(0);
        assert_eq!(row, "1\t0\t\\N\n");

        let row = generate_branch_row(9);
        assert_eq!(row, "10\t0\t\\N\n");
    }

    #[test]
    fn test_generate_teller_row() {
        let row = generate_teller_row(0);
        assert_eq!(row, "1\t1\t0\t\\N\n");

        // Teller 10 should be in branch 2
        let row = generate_teller_row(10);
        assert_eq!(row, "11\t2\t0\t\\N\n");
    }

    #[test]
    fn test_generate_account_row() {
        let row = generate_account_row(0);
        assert_eq!(row, "1\t1\t0\t\n");

        // Account 100,000 should be in branch 2
        let row = generate_account_row(100_000);
        assert_eq!(row, "100001\t2\t0\t\n");
    }

    #[test]
    fn test_scaling_calculations() {
        let scale = 10i64;
        assert_eq!(BRANCHES_PER_SCALE * scale, 10);
        assert_eq!(TELLERS_PER_SCALE * scale, 100);
        assert_eq!(ACCOUNTS_PER_SCALE * scale, 1_000_000);
    }
}
