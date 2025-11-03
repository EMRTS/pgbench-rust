# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust port of PostgreSQL's `pgbench` benchmarking tool. The goal is 100% feature and command-line compatibility with the original C implementation while leveraging Rust's memory safety guarantees.

**Current Status**: Initial setup complete (Phase 1). Core structure in place with stub implementations.

## Build and Development Commands

```bash
# Check code compiles
cargo check

# Run tests
cargo test

# Run specific test
cargo test test_name

# Format code
cargo fmt

# Lint (follow suggestions)
cargo clippy

# Build release version (optimized for benchmarking)
cargo build --release

# Run with debug logging
RUST_LOG=debug cargo run -- --help
```

## Project Structure

- `src/cli.rs` - Command-line parsing (clap-based, already implemented)
- `src/db/` - Database operations (connection, initialization, query execution)
- `src/expr/` - Expression parser and evaluator (port from Bison/Flex grammar)
- `src/script/` - Transaction script parsing and execution
- `src/random/` - PRNG and statistical distributions (must match C implementation)
- `src/worker/` - Multi-threaded workload execution
- `src/stats/` - Statistics collection and reporting
- `src/types.rs` - Core data structures
- `src/error.rs` - Error types

## Architecture & Key Design Decisions

### Threading Model
- Use native threads (`std::thread`) to match C behavior
- Thread barrier synchronization for coordinated start (`std::sync::Barrier`)
- Per-thread statistics (lock-free during collection, aggregate at end)
- Each thread manages multiple database connections (clients/threads ratio)

### Expression Parser
The original uses Bison/Flex. Choose one approach:
- **pest** (current default): PEG parser, easier to maintain
- **nom**: Parser combinators, better performance
- **lalrpop**: LALR parser generator, closest to Bison

Grammar is in `original-source/exprparse.y` and `exprscan.l`. Must support:
- Variables (`:varname`)
- Arithmetic, comparison, logical, bitwise operators
- Functions (random, hash, sqrt, abs, etc.)
- CASE expressions

### Random Number Generation
**CRITICAL**: Must use Xoroshiro128** algorithm for exact compatibility with pg_prng.
- Implement custom RNG using `rand` crate infrastructure
- Same seed must produce same sequence (reproducibility requirement)
- Support distributions: uniform, Gaussian, exponential, Zipfian
- Located in `src/random/prng.rs` and `src/random/distributions.rs`

### Database Operations
- Start with synchronous `postgres` crate (not async)
- Connection pooling: per-thread, reuse connections
- Prepared statement caching for performance
- Schema matches original exactly (see ARCHITECTURE.md for table definitions)

### Error Handling
- Use `Result<T, PgBenchError>` throughout
- Use `?` operator for clean propagation
- Provide context in errors (line numbers, file names)
- Fatal vs transient errors: connection failures exit, query errors may retry

## Development Workflow

### Porting Priority (from PORTING_PLAN.md)
Current phase: **Phase 1** (Week 1-2)
- [x] Project setup
- [ ] CLI argument parsing (mostly done)
- [ ] Database connection (`src/db/connection.rs`)
- [ ] Basic logging (infrastructure exists)

Next phases:
- **Phase 2**: Core data structures in `src/types.rs`
- **Phase 3**: Expression parser (most complex module)
- **Phase 4**: Database operations
- **Phase 5**: Random number generation
- **Phase 6**: Multi-threading
- **Phase 7**: Statistics and reporting

### Reference Material
Always consult these when implementing:
- `original-source/pgbench.c` - Main C implementation (8,015 lines)
- `original-source/pgbench.h` - Type definitions
- `original-source/exprparse.y` - Expression grammar
- `original-source/exprscan.l` - Expression lexer
- `DEPENDENCIES.md` - C to Rust dependency mapping
- `ARCHITECTURE.md` - Detailed design and data flow

### Testing Strategy
- Unit tests in each module (`#[cfg(test)]`)
- Integration tests in `tests/` (mark with `#[ignore]` if PostgreSQL required)
- Property tests for RNG using `proptest`
- Compatibility tests: compare output with original pgbench

## Key Compatibility Requirements

1. **Command-line interface**: All flags must match original pgbench exactly
2. **Expression language**: Support all syntax from original
3. **Random sequences**: Same seed → same random values (critical for reproducibility)
4. **Output format**: Match original format for easy comparison
5. **Performance**: Within 10% of C version

## Common Patterns

### Database Connection String
```rust
// Example: postgres://user:password@localhost/dbname
// Or: postgres://localhost/pgbench_test
```

### Built-in Transaction Scripts
Default is TPC-B-like. Also support:
- `simple-update`: UPDATE only
- `select-only`: SELECT only
- Custom scripts via `-f` flag

### Meta-commands in Scripts
- `\set variable expression` - Set variable
- `\sleep time` - Sleep (microseconds)
- `\if expression` - Conditional execution
- `\elif`, `\else`, `\endif` - Conditional blocks
- `\setshell variable command` - Set from shell output
- `\startpipeline`, `\endpipeline` - Pipeline mode

## Performance Considerations

### Hot Paths (optimize these)
1. Transaction execution loop
2. Expression evaluation
3. Random number generation
4. Database I/O (inherent bottleneck)

### Optimization Strategies
- Connection pooling and reuse
- Prepared statement caching
- Lock-free per-thread statistics
- Avoid allocations in hot paths
- Let compiler inline small functions

## Dependency Mapping (C → Rust)

Key replacements from C code:
- `libpq` → `postgres` crate
- `pthread` → `std::thread` + `std::sync`
- `getopt_long` → `clap` (already implemented)
- `pg_prng` → custom Xoroshiro128** implementation
- `instr_time` → `std::time::Instant`
- `pg_log_*` → `log` + `env_logger`
- Integer overflow checking → `i64::checked_add()`, etc.

See DEPENDENCIES.md for comprehensive mapping.

## Code Style Notes

- Follow standard Rust conventions (enforced by `cargo fmt`)
- Use `Result<T, E>` instead of panicking
- Prefer `?` operator over `.unwrap()`
- Document public APIs with `///` comments
- Keep functions focused and testable
- Use descriptive variable names

## Common Pitfalls

1. **Don't guess at C behavior**: Always reference original source
2. **RNG must match exactly**: Can't substitute random algorithms
3. **Thread safety**: Per-thread state, minimal shared state
4. **Error handling**: Transaction errors may be expected (serialization failures)
5. **SQL dialect**: PostgreSQL-specific features may be used
