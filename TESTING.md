# Testing pgbench-rust

This guide explains how to set up PostgreSQL and test pgbench-rust end-to-end.

## Prerequisites

- Rust toolchain (cargo, rustc)
- PostgreSQL server (see setup options below)

## PostgreSQL Setup

You have two options: Docker (recommended) or local PostgreSQL installation.

### Option 1: Docker (Recommended - Easiest)

**Start PostgreSQL in Docker:**

```bash
# Start PostgreSQL container
docker run --name pgbench-postgres \
  -e POSTGRES_PASSWORD=pgbench \
  -e POSTGRES_USER=pgbench \
  -e POSTGRES_DB=pgbench \
  -p 5432:5432 \
  -d postgres:16

# Wait a few seconds for PostgreSQL to start
sleep 5

# Test the connection
psql postgresql://pgbench:pgbench@localhost:5432/pgbench -c "SELECT version();"
```

**Connection String:**
```
postgresql://pgbench:pgbench@localhost:5432/pgbench
```

**Useful Docker Commands:**

```bash
# Stop the container
docker stop pgbench-postgres

# Start the container again
docker start pgbench-postgres

# Remove the container (deletes all data)
docker rm -f pgbench-postgres

# View PostgreSQL logs
docker logs pgbench-postgres
```

### Option 2: Local PostgreSQL Installation

**Ubuntu/Debian:**

```bash
# Install PostgreSQL
sudo apt-get update
sudo apt-get install postgresql postgresql-client

# Create user and database
sudo -u postgres psql -c "CREATE USER pgbench WITH PASSWORD 'pgbench';"
sudo -u postgres psql -c "CREATE DATABASE pgbench OWNER pgbench;"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE pgbench TO pgbench;"
```

**macOS:**

```bash
# Install PostgreSQL
brew install postgresql@16

# Start PostgreSQL service
brew services start postgresql@16

# Create user and database
createuser -s pgbench
psql postgres -c "ALTER USER pgbench WITH PASSWORD 'pgbench';"
createdb -O pgbench pgbench
```

**Connection String:**
```
postgresql://pgbench:pgbench@localhost:5432/pgbench
```

## Running Tests

### 1. Unit Tests

Run all unit tests (no database required):

```bash
cargo test
```

Expected output: `240 passed; 0 failed; 10 ignored`

### 2. Database Initialization

Initialize the pgbench database schema and populate with test data:

```bash
# Default scale factor (1)
cargo run -- -i postgresql://pgbench:pgbench@localhost:5432/pgbench

# Custom scale factor (creates 10x more data)
cargo run -- -i -s 10 postgresql://pgbench:pgbench@localhost:5432/pgbench

# With partitions
cargo run -- -i --partitions 4 --partition-method hash \
  postgresql://pgbench:pgbench@localhost:5432/pgbench
```

**What gets created:**
- `pgbench_branches` - 1 row per scale unit
- `pgbench_tellers` - 10 rows per scale unit
- `pgbench_accounts` - 100,000 rows per scale unit
- `pgbench_history` - empty table for transaction history

### 3. Quick Benchmark Test

Run a minimal benchmark to verify everything works:

```bash
# 10 transactions, 1 client, 1 thread
cargo run -- -t 10 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

Expected output:
```
transaction type: <builtin: TPC-B (sort of)>
scaling factor: 1
query mode: simple
number of clients: 1
number of threads: 1
duration: X s
number of transactions actually processed: 10
latency average = X.XXX ms
latency stddev = X.XXX ms
tps = X.XXXXXX (including connections establishing)
```

### 4. Multi-threaded Benchmark

Test multi-threading capabilities:

```bash
# 100 transactions, 4 clients, 2 threads
cargo run -- -c 4 -j 2 -t 100 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

### 5. Time-limited Benchmark

Run for a specific duration:

```bash
# Run for 10 seconds with 2 clients
cargo run -- -T 10 -c 2 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

### 6. Latency Reporting

Get detailed percentile statistics:

```bash
# Show P50, P90, P95, P99 latencies
cargo run -- -t 50 --report-latencies \
  postgresql://pgbench:pgbench@localhost:5432/pgbench
```

Expected additional output:
```
latency statistics:
         min: X.XXX ms
         max: X.XXX ms
         p50: X.XXX ms
         p90: X.XXX ms
         p95: X.XXX ms
         p99: X.XXX ms
```

### 7. Quiet Mode

Minimal output for scripting:

```bash
# Just show transaction count and TPS
cargo run -- -q -t 20 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

Expected output:
```
20 transactions (XX.XX tps)
```

### 8. Debug Mode

Verbose logging for troubleshooting:

```bash
# Enable debug logging
cargo run -- -d -t 5 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

### 9. Different Query Modes

Test different PostgreSQL protocol modes:

```bash
# Simple protocol (default)
cargo run -- -M simple -t 10 postgresql://pgbench:pgbench@localhost:5432/pgbench

# Extended protocol
cargo run -- -M extended -t 10 postgresql://pgbench:pgbench@localhost:5432/pgbench

# Prepared statements
cargo run -- -M prepared -t 10 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

### 10. Skip VACUUM

Skip the automatic VACUUM before benchmarking:

```bash
# Faster startup, but may have inconsistent results
cargo run -- -n -t 10 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

### 11. Custom Random Seed

Reproducible results with a fixed seed:

```bash
# Same seed produces same random sequence
cargo run -- --random-seed 12345 -t 10 \
  postgresql://pgbench:pgbench@localhost:5432/pgbench
```

## Comparing with Original pgbench

If you have the original C pgbench installed, you can compare:

```bash
# Initialize with C pgbench
pgbench -i postgresql://pgbench:pgbench@localhost:5432/pgbench

# Run C pgbench
pgbench -t 10 postgresql://pgbench:pgbench@localhost:5432/pgbench

# Run Rust pgbench (should produce similar results)
cargo run -- -t 10 postgresql://pgbench:pgbench@localhost:5432/pgbench
```

## Verifying Database State

Check what was created:

```bash
# Connect to database
psql postgresql://pgbench:pgbench@localhost:5432/pgbench

# List tables
\dt

# Count rows
SELECT 'branches' as table, count(*) FROM pgbench_branches
UNION ALL
SELECT 'tellers', count(*) FROM pgbench_tellers
UNION ALL
SELECT 'accounts', count(*) FROM pgbench_accounts
UNION ALL
SELECT 'history', count(*) FROM pgbench_history;

# Check a sample account
SELECT * FROM pgbench_accounts LIMIT 5;
```

## Troubleshooting

### Connection Refused

```bash
# Check if PostgreSQL is running
pg_isready -h localhost

# For Docker, check container status
docker ps | grep pgbench-postgres
```

### Permission Denied

```bash
# Make sure the user has proper permissions
psql postgresql://pgbench:pgbench@localhost:5432/pgbench -c "
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO pgbench;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO pgbench;
"
```

### Tables Already Exist

```bash
# Drop existing tables and reinitialize
psql postgresql://pgbench:pgbench@localhost:5432/pgbench -c "
DROP TABLE IF EXISTS pgbench_history;
DROP TABLE IF EXISTS pgbench_accounts CASCADE;
DROP TABLE IF EXISTS pgbench_tellers;
DROP TABLE IF EXISTS pgbench_branches;
"

# Then reinitialize
cargo run -- -i postgresql://pgbench:pgbench@localhost:5432/pgbench
```

## Performance Testing

For more realistic performance testing:

```bash
# Larger scale factor (1M accounts)
cargo run --release -- -i -s 10 postgresql://pgbench:pgbench@localhost:5432/pgbench

# Longer benchmark with multiple clients
cargo run --release -- -c 8 -j 4 -T 60 \
  postgresql://pgbench:pgbench@localhost:5432/pgbench

# With latency reporting
cargo run --release -- -c 4 -j 2 -T 30 --report-latencies \
  postgresql://pgbench:pgbench@localhost:5432/pgbench
```

**Note:** Use `cargo run --release` for performance testing to enable optimizations.

## Integration Tests

Run the ignored integration tests (requires PostgreSQL):

```bash
# Set connection string environment variable
export PGBENCH_TEST_CONN="postgresql://pgbench:pgbench@localhost:5432/pgbench"

# Run all tests including ignored ones
cargo test -- --ignored --test-threads=1
```

## Cleanup

### Docker

```bash
# Stop and remove container
docker rm -f pgbench-postgres
```

### Local PostgreSQL

```bash
# Drop database
dropdb pgbench

# Or via SQL
psql postgres -c "DROP DATABASE pgbench;"
```

## Next Steps

- Try different scale factors to see performance characteristics
- Compare results with original C pgbench
- Test with different PostgreSQL versions (10, 11, 12, 13, 14, 15, 16)
- Profile with different client/thread combinations
- Test custom transaction scripts (not yet implemented)

## Known Limitations

Current version supports:
- ✅ Database initialization
- ✅ Built-in TPC-B-like benchmark
- ✅ Multi-threaded execution
- ✅ Transaction and time limits
- ✅ Latency statistics and percentiles
- ✅ Query protocol modes (simple, extended, prepared)
- ✅ Random seed control
- ✅ Quiet and debug modes

Not yet implemented:
- ⏳ Custom transaction scripts (-f flag)
- ⏳ Progress reporting (-P flag)
- ⏳ Connection establishment mode (-C flag)
- ⏳ Rate limiting (--rate flag)
- ⏳ Per-transaction logging (--log flag)
- ⏳ Built-in scripts: simple-update, select-only (-b flag)

See PLAN.md for the full implementation roadmap.
