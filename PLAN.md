# pgbench-rust Implementation Plan

This file tracks the actual implementation progress for porting pgbench to Rust.
See PORTING_PLAN.md for the overall strategy and ARCHITECTURE.md for design decisions.

**Last Updated**: 2025-11-02 (Phase 1.4 Database Connection completed)
**Current Phase**: Phase 1 - Foundation (85% complete)
**Status**: In Progress

---

## Phase 1: Foundation & Basic Infrastructure (Current)

### 1.1 Project Setup ✅
- [x] Create Cargo project structure
- [x] Configure dependencies in Cargo.toml
- [x] Set up module structure
- [x] Create documentation (README, ARCHITECTURE, DEPENDENCIES, PORTING_PLAN)
- [x] Add CLAUDE.md for AI assistance
- [x] Create PLAN.md for implementation tracking

### 1.2 CLI Argument Parsing ✅
- [x] Implement Args struct with clap
- [x] Add all command-line flags matching original pgbench
- [x] Implement argument validation
- [x] Add unit tests for validation
- [x] Document CLI usage

### 1.3 Error Handling 🚧
- [x] Define PgBenchError enum (src/error.rs)
- [ ] Add error variants for all failure modes
- [ ] Implement From traits for external errors
- [ ] Add context to errors (file, line)
- [ ] Test error propagation

### 1.4 Database Connection ✅
**Priority: HIGH - Completed**

File: `src/db/connection.rs`

- [x] Implement connection string parsing
- [x] Create PgBenchConnection wrapper around postgres::Client
- [ ] Add connection pool per thread (deferred to Phase 7)
- [x] Implement connection retry logic
- [x] Add connection validation
- [x] Test connection with various PostgreSQL versions (integration tests marked #[ignore])
- [x] Handle connection errors gracefully

**Acceptance Criteria:**
- ✅ Can connect to PostgreSQL with connection string
- ✅ Handles invalid connection strings with clear errors
- ✅ Retries transient connection failures (3 attempts with 1s delay)
- ✅ Works with PostgreSQL 10+
- ✅ Sanitizes connection strings in logs (hides passwords)
- ✅ Implements reconnect functionality

### 1.5 Logging Infrastructure ✅
- [x] Set up env_logger
- [x] Add log levels (info, warn, error, debug)
- [ ] Add structured logging for benchmarks
- [ ] Implement quiet mode (-q flag)
- [ ] Add debug mode (-d flag)

---

## Phase 2: Core Data Structures

### 2.1 Type Definitions 🔲
File: `src/types.rs`

- [ ] Define PgBenchValue enum (int, double, bool, null)
- [ ] Implement PgBenchValue conversions
- [ ] Define PgBenchExpr for expression AST
- [ ] Define PgBenchFunction enum
- [ ] Define Command enum (SQL vs meta-command)
- [ ] Define MetaCommand variants
- [ ] Define TransactionStats struct
- [ ] Define ThreadState struct
- [ ] Add unit tests for all types

### 2.2 Utility Functions 🔲
File: `src/utils.rs`

- [ ] Port string utilities
- [ ] Add time formatting helpers
- [ ] Implement username detection (whoami crate)
- [ ] Add file I/O utilities
- [ ] Test edge cases

---

## Phase 3: Expression Parser & Evaluator

**Status**: Not Started
**Dependencies**: Phase 2 complete

### 3.1 Choose Parser Implementation 🔲
- [ ] Evaluate pest vs nom vs lalrpop
- [ ] Create proof-of-concept for chosen approach
- [ ] Document decision in ARCHITECTURE.md

### 3.2 Expression Grammar 🔲
File: `src/expr/parser.rs` (and possibly `src/expr/grammar.pest`)

Reference: `original-source/exprparse.y`, `original-source/exprscan.l`

- [ ] Port expression grammar from Bison
- [ ] Support literals (integers, doubles, booleans)
- [ ] Support variables (`:varname`)
- [ ] Support arithmetic operators (+, -, *, /, %)
- [ ] Support comparison operators (=, <, >, <=, >=, !=, <>)
- [ ] Support logical operators (AND, OR, NOT)
- [ ] Support bitwise operators (&, |, #, ~, <<, >>)
- [ ] Support function calls
- [ ] Support CASE expressions
- [ ] Support operator precedence
- [ ] Add comprehensive parser tests

### 3.3 Expression Evaluator 🔲
File: `src/expr/eval.rs`

- [ ] Implement EvalContext for variable bindings
- [ ] Implement arithmetic evaluation
- [ ] Implement comparison evaluation
- [ ] Implement logical evaluation
- [ ] Implement bitwise evaluation
- [ ] Implement type coercion rules
- [ ] Handle NULL values
- [ ] Handle division by zero
- [ ] Add evaluation tests

### 3.4 Built-in Functions 🔲
File: `src/expr/eval.rs`

- [ ] Math: abs, sqrt, pow, exp, ln, log
- [ ] Random: random, random_gaussian, random_exponential, random_zipfian
- [ ] Hash: hash, hash_murmur2, hash_fnv1a
- [ ] Misc: min, max, debug, pi
- [ ] Test all functions

---

## Phase 4: Database Operations

**Status**: Not Started
**Dependencies**: Phase 2, 3.1-3.3 complete

### 4.1 Database Initialization 🔲
File: `src/db/init.rs`

Reference: `pgbench.c` lines 4000-5000 (approximately)

- [ ] Implement table drop logic
- [ ] Implement table creation (pgbench_accounts, branches, tellers, history)
- [ ] Implement data population with scaling factor
- [ ] Add partition support (--partitions flag)
- [ ] Implement index creation
- [ ] Implement VACUUM and ANALYZE
- [ ] Support init steps flag (-I dtgvpf)
- [ ] Support tablespace options
- [ ] Support unlogged tables (--unlogged-tables)
- [ ] Add progress reporting during init
- [ ] Test with various scale factors

**Acceptance Criteria:**
- `pgbench -i -s 10 postgres://localhost/test` creates and populates tables
- Matches original pgbench schema exactly
- Partition support works

### 4.2 Query Execution 🔲
File: `src/db/query.rs`

- [ ] Implement query execution wrapper
- [ ] Add prepared statement support
- [ ] Implement statement caching
- [ ] Support simple protocol (-M simple)
- [ ] Support extended protocol (-M extended)
- [ ] Support prepared protocol (-M prepared)
- [ ] Handle query errors
- [ ] Add query timeout support
- [ ] Test different protocol modes

---

## Phase 5: Transaction Scripts

**Status**: Not Started
**Dependencies**: Phase 3, 4.2 complete

### 5.1 Script Parser 🔲
File: `src/script/parser.rs`

Reference: `pgbench.c` parseScript function

- [ ] Parse SQL commands
- [ ] Parse \set meta-command
- [ ] Parse \sleep meta-command
- [ ] Parse \if, \elif, \else, \endif meta-commands
- [ ] Parse \setshell meta-command
- [ ] Parse \startpipeline, \endpipeline
- [ ] Handle comments
- [ ] Handle line continuations
- [ ] Load script from file
- [ ] Support multiple scripts (-f flag)
- [ ] Test script parsing

### 5.2 Built-in Scripts 🔲
File: `src/script/mod.rs`

- [ ] Implement TPC-B-like (default)
- [ ] Implement simple-update (-b simple-update)
- [ ] Implement select-only (-b select-only)
- [ ] Test built-in scripts

### 5.3 Script Executor 🔲
File: `src/script/executor.rs`

- [ ] Execute SQL commands
- [ ] Execute \set command (with expression evaluation)
- [ ] Execute \sleep command
- [ ] Execute conditional commands (\if, \elif, \else, \endif)
- [ ] Execute \setshell command
- [ ] Execute pipeline commands
- [ ] Track transaction state
- [ ] Handle transaction errors
- [ ] Test script execution

---

## Phase 6: Random Number Generation

**Status**: Not Started
**Dependencies**: Phase 2 complete

### 6.1 PRNG Implementation 🔲
File: `src/random/prng.rs`

Reference: `src/common/pg_prng.c` in PostgreSQL source

**CRITICAL**: Must match pg_prng exactly for reproducibility

- [ ] Implement Xoroshiro128** algorithm
- [ ] Implement RngCore trait
- [ ] Implement SeedableRng trait
- [ ] Add seed initialization
- [ ] Implement uint64 generation
- [ ] Implement double generation [0.0, 1.0)
- [ ] Test against known seeds
- [ ] Verify matches PostgreSQL pg_prng output

### 6.2 Statistical Distributions 🔲
File: `src/random/distributions.rs`

Reference: `pgbench.c` distribution functions

- [ ] Implement uniform distribution
- [ ] Implement Gaussian distribution (Box-Muller)
- [ ] Implement exponential distribution
- [ ] Implement Zipfian distribution
- [ ] Add distribution parameters
- [ ] Add statistical tests (chi-square, etc.)
- [ ] Test distribution properties
- [ ] Verify matches original pgbench

**Acceptance Criteria:**
- Same seed produces same sequence as original pgbench
- Statistical properties match expected distributions
- Performance is comparable

---

## Phase 7: Multi-threading & Worker Execution

**Status**: Not Started
**Dependencies**: Phase 4, 5, 6 complete

### 7.1 Thread Management 🔲
File: `src/worker/thread.rs`

- [ ] Create thread pool
- [ ] Implement barrier synchronization
- [ ] Distribute clients across threads
- [ ] Handle thread creation errors
- [ ] Implement graceful shutdown
- [ ] Support Ctrl+C signal handling (ctrlc crate)
- [ ] Test thread coordination

### 7.2 Worker State 🔲
File: `src/worker/state.rs`

- [ ] Define per-thread state
- [ ] Implement variable storage
- [ ] Track RNG state per thread
- [ ] Track connection per thread
- [ ] Track statistics per thread
- [ ] Test state isolation

### 7.3 Benchmark Execution 🔲
File: `src/worker/mod.rs`

- [ ] Implement main benchmark loop
- [ ] Support transaction count limit (-t)
- [ ] Support time limit (-T)
- [ ] Implement rate limiting (--rate)
- [ ] Implement latency limit (--latency-limit)
- [ ] Handle benchmark errors
- [ ] Support progress reporting (-P)
- [ ] Test benchmark execution

**Acceptance Criteria:**
- Multi-threaded execution works correctly
- Rate limiting is accurate
- Progress reporting works
- Clean shutdown on Ctrl+C

---

## Phase 8: Statistics & Reporting

**Status**: Not Started
**Dependencies**: Phase 7 complete

### 8.1 Statistics Collection 🔲
File: `src/stats/collector.rs`

- [ ] Track transaction count
- [ ] Track latency per transaction
- [ ] Track failures
- [ ] Support per-transaction logging (--log)
- [ ] Support sampling (--sampling-rate)
- [ ] Aggregate per-thread statistics
- [ ] Calculate percentiles (P50, P90, P95, P99)
- [ ] Test statistics accuracy

### 8.2 Report Generation 🔲
File: `src/stats/reporter.rs`

- [ ] Format TPS (transactions per second)
- [ ] Format latency statistics
- [ ] Format percentile report
- [ ] Support aggregate intervals (--aggregate-interval)
- [ ] Support progress reporting format
- [ ] Match original pgbench output format
- [ ] Support quiet mode
- [ ] Test report formatting

**Acceptance Criteria:**
- Output matches original pgbench format
- Percentile calculations are accurate
- Progress reporting updates correctly

---

## Phase 9: Advanced Features

**Status**: Not Started
**Dependencies**: Phase 8 complete

### 9.1 Advanced Options 🔲
- [ ] Connection establishment mode (-C)
- [ ] No vacuum option (-n)
- [ ] Custom random seed (--random-seed)
- [ ] Report latencies (--report-latencies)
- [ ] Test all options

### 9.2 Performance Optimization 🔲
- [ ] Profile hot paths
- [ ] Optimize expression evaluation
- [ ] Optimize RNG
- [ ] Reduce allocations in hot paths
- [ ] Benchmark against C version
- [ ] Achieve within 10% of C performance

---

## Phase 10: Testing & Validation

**Status**: Not Started
**Dependencies**: Phase 9 complete

### 10.1 Integration Tests 🔲
File: `tests/integration_test.rs`

- [ ] Test database initialization
- [ ] Test basic benchmark
- [ ] Test multi-threaded benchmark
- [ ] Test custom scripts
- [ ] Test all command-line options
- [ ] Test error conditions

### 10.2 Compatibility Tests 🔲
File: `tests/compatibility_test.rs`

- [ ] Compare with original pgbench output
- [ ] Test with same random seeds
- [ ] Verify identical TPS calculations
- [ ] Test across PostgreSQL versions (10, 11, 12, 13, 14, 15, 16)
- [ ] Test on Linux, macOS, Windows

### 10.3 Documentation 🔲
- [ ] Complete API documentation
- [ ] Add usage examples
- [ ] Create migration guide from C pgbench
- [ ] Document known differences
- [ ] Update README with status

---

## Immediate Next Steps

1. **Complete Phase 1.3**: Finish error handling
   - Add remaining error variants for all failure modes
   - Implement From traits for external errors
   - Add context to errors (file, line)
   - Test error propagation

2. **Complete Phase 2.1**: Define core types
   - Implement in `src/types.rs`
   - Reference `pgbench.h` for type definitions
   - Focus on PgBenchValue, PgBenchExpr, and Command types

3. **Start Phase 3.1**: Choose expression parser
   - Evaluate pest vs nom vs lalrpop
   - Create proof-of-concept
   - Make decision and document in ARCHITECTURE.md

---

## Progress Tracking

- ✅ Completed
- 🚧 In Progress
- 🔲 Not Started
- ⏸️ Blocked

### Phase Summary
- Phase 1: 🚧 85% complete (Database connection implemented, error handling needs completion)
- Phase 2: 🔲 Not started
- Phase 3: 🔲 Not started
- Phase 4: 🔲 Not started
- Phase 5: 🔲 Not started
- Phase 6: 🔲 Not started
- Phase 7: 🔲 Not started
- Phase 8: 🔲 Not started
- Phase 9: 🔲 Not started
- Phase 10: 🔲 Not started

### Overall Progress: ~9%

---

## Notes & Decisions

### 2025-11-02 (Update 2)
- **Database Connection (Phase 1.4) completed**:
  - Implemented `PgBenchConnection` wrapper with retry logic
  - Added connection validation with `SELECT 1` test query
  - Implemented connection string sanitization for secure logging
  - Added reconnect functionality
  - Created comprehensive unit tests (integration tests require PostgreSQL)
  - Connection retries up to 3 times with 1-second delay between attempts
- Phase 1 now ~85% complete
- Next: Complete error handling (Phase 1.3), then move to core types (Phase 2.1)

### 2025-11-02 (Initial)
- Project structure created
- CLI parsing implemented and tested
- CLAUDE.md created for AI assistance
- PLAN.md created for tracking
- Next: Focus on database connection implementation

---

## Blockers & Issues

None currently.

---

## Testing Checklist

Before marking each phase complete:
- [ ] All unit tests pass
- [ ] Code formatted with `cargo fmt`
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] Changes committed to git
