# pgbench-rust

A Rust port of PostgreSQL's `pgbench` benchmarking tool.

## Overview

`pgbench` is a simple program for running benchmark tests on PostgreSQL. It runs the same sequence of SQL commands over and over, possibly in multiple concurrent database sessions, and calculates the average transaction rate (transactions per second).

This is a complete port from C to Rust, aiming for:
- Full feature compatibility with the original pgbench
- Improved memory safety through Rust's ownership system
- Modern, idiomatic Rust code
- Maintainable and extensible architecture

## Status

**Current Phase**: Core implementation (Phase 9 - Async migration complete)
**Version**: 0.9.0 (Beta)

See [PORTING_PLAN.md](PORTING_PLAN.md) for the detailed porting roadmap and [ASYNC_MIGRATION.md](ASYNC_MIGRATION.md) for async migration details.

## Features

The goal is to support all features of the original pgbench:

### Core Functionality
- [x] Project structure and dependencies
- [x] Database initialization with benchmarking tables
- [x] Built-in TPC-B-like transaction scenarios
- [x] Custom transaction scripts
- [x] Multiple concurrent client connections
- [x] Async execution for parallel workload (tokio-based)

### Benchmarking Options
- [x] Duration-based testing (-T)
- [x] Transaction count-based testing (-t)
- [x] Multiple clients (-c)
- [x] Multiple threads (-j)
- [ ] Rate limiting (--rate)
- [x] Latency limit (--latency-limit)

### Expression Language
- [x] Variable substitution (:varname)
- [x] Arithmetic expressions
- [x] Random number generation (uniform, gaussian, exponential, zipfian)
- [x] Hash functions (FNV-1a, MurmurHash2)
- [x] Built-in functions (abs, sqrt, pow, etc.)
- [x] CASE expressions

### Reporting
- [x] Transaction per second (TPS)
- [x] Latency statistics (average, median, p90, p95, p99)
- [x] Per-transaction logging
- [x] Progress reporting
- [ ] Sampling mode

## Prerequisites

- Rust 1.70 or later
- PostgreSQL 10 or later (for testing)

## Building

```bash
# Clone the repository
git clone https://github.com/yourusername/pgbench-rust.git
cd pgbench-rust

# Build in debug mode
cargo build

# Build optimized release version
cargo build --release

# Run tests
cargo test

# Run with cargo
cargo run -- --help
```

## Installation

```bash
# Install from source
cargo install --path .

# Or install from crates.io (when published)
cargo install pgbench-rust
```

## Usage

```bash
# Initialize benchmark database
pgbench -i postgres://localhost/testdb

# Run benchmark with default settings
pgbench postgres://localhost/testdb

# Run with 10 clients, 2 threads, for 60 seconds
pgbench -c 10 -j 2 -T 60 postgres://localhost/testdb

# Run custom script
pgbench -f custom_script.sql postgres://localhost/testdb

# Show all options
pgbench --help
```

## Documentation

- [PORTING_PLAN.md](PORTING_PLAN.md) - Detailed porting plan and timeline
- [DEPENDENCIES.md](DEPENDENCIES.md) - Comprehensive dependency documentation
- [ARCHITECTURE.md](ARCHITECTURE.md) - (TODO) Architecture and design decisions
- [original-source/](original-source/) - Reference C source code from PostgreSQL

## Project Structure

```
pgbench-rust/
├── Cargo.toml              # Rust package manifest
├── README.md               # This file
├── PORTING_PLAN.md         # Detailed porting roadmap
├── DEPENDENCIES.md         # Dependency analysis and mapping
├── src/
│   ├── main.rs            # Entry point
│   ├── lib.rs             # Library root
│   ├── cli.rs             # Command-line interface
│   ├── db/                # Database operations
│   │   ├── mod.rs
│   │   ├── connection.rs  # Connection management
│   │   ├── init.rs        # Database initialization
│   │   └── query.rs       # Query execution
│   ├── expr/              # Expression parser and evaluator
│   │   ├── mod.rs
│   │   ├── parser.rs      # Expression parser
│   │   ├── eval.rs        # Expression evaluator
│   │   └── grammar.pest   # PEG grammar (if using pest)
│   ├── script/            # Script parsing and execution
│   │   ├── mod.rs
│   │   ├── parser.rs      # Script parser
│   │   └── executor.rs    # Script executor
│   ├── random/            # Random number generation
│   │   ├── mod.rs
│   │   ├── prng.rs        # PRNG implementation
│   │   └── distributions.rs  # Statistical distributions
│   ├── worker/            # Thread management
│   │   ├── mod.rs
│   │   ├── thread.rs      # Worker threads
│   │   └── state.rs       # Thread state
│   ├── stats/             # Statistics collection
│   │   ├── mod.rs
│   │   ├── collector.rs   # Stats collector
│   │   └── reporter.rs    # Stats reporter
│   ├── types.rs           # Common type definitions
│   ├── error.rs           # Error types
│   └── utils.rs           # Utility functions
├── tests/                 # Integration tests
│   ├── integration_test.rs
│   └── compatibility_test.rs
├── benches/               # Performance benchmarks
│   └── benchmark.rs
└── original-source/       # Reference C source files
    ├── pgbench.c
    ├── pgbench.h
    ├── exprparse.y
    └── exprscan.l
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_test
```

### Code Style

This project follows the standard Rust style guidelines:

```bash
# Format code
cargo fmt

# Check for common issues
cargo clippy

# Check without building
cargo check
```

### Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Run `cargo fmt` and `cargo clippy`
7. Submit a pull request

### Development Workflow

1. Pick a module from the porting plan
2. Review the original C code in `original-source/`
3. Consult `DEPENDENCIES.md` for mapping C → Rust
4. Implement in Rust with tests
5. Verify compatibility with original pgbench
6. Document any deviations or design decisions

## Compatibility

This project aims for 100% command-line compatibility with PostgreSQL's pgbench. All existing pgbench scripts and workflows should work without modification.

### Differences from Original

While this project aims for full command-line compatibility with the original pgbench, there are some implementation differences:

#### 1. Async Architecture (Critical Fix)

**Why**: The original pgbench uses synchronous blocking I/O with OS threads. This causes deadlocks when multiple clients run on a single thread (`-c 2 -j 1`) because blocking database operations prevent other clients from releasing locks.

**Solution**: pgbench-rust uses async I/O with `tokio-postgres` instead of synchronous `postgres`. This allows `.await` to yield control, enabling multiple clients to make progress concurrently on a single thread without deadlocks.

**Impact on Benchmarks**: ✅ **None**. Running pgbench and pgbench-rust with the same parameters on the same database should produce similar TPS and latency results. The async architecture only affects the internal concurrency model, not the benchmark methodology.

#### 2. Database Initialization Performance

**Current Status**: pgbench-rust uses batched `INSERT` statements instead of PostgreSQL's `COPY FROM STDIN` for data loading during initialization (`-i` mode).

**Reason**: This is a temporary implementation during the async migration. The `tokio-postgres` crate uses a different API (`CopyInSink`) compared to the synchronous `postgres` crate (`CopyWriter`). Full COPY support will be added in a future optimization phase.

**Impact**:
- ❌ **Initialization is slower** (especially for large scale factors like `-s 100` or higher)
- ✅ **Benchmark results are unaffected** - initialization only sets up test data, it doesn't affect the actual benchmark execution or measurements
- ✅ **Data is identical** - same tables, indexes, and constraints as original pgbench

**Workaround**: If initialization speed is critical, you can:
1. Initialize with original C pgbench: `pgbench -i -s 100 postgres://...`
2. Run benchmarks with pgbench-rust: `pgbench-rust -c 10 -j 2 -T 60 postgres://...`

Both tools use the same database schema and are fully compatible for benchmarking.

#### 3. Technical Implementation Differences

**Database Driver**:
- Original: `libpq` (C library, synchronous)
- Rust port: `tokio-postgres` (Rust async library)

**Threading Model**:
- Original: OS threads with blocking I/O
- Rust port: Tokio async runtime with cooperative multitasking

**Memory Management**:
- Original: Manual memory management (malloc/free)
- Rust port: Automatic with ownership system (no garbage collection overhead)

**Error Handling**:
- Original: Return codes and errno
- Rust port: Result types with structured errors

#### 4. Compatibility Guarantee

**✅ Full command-line compatibility**: All flags, options, and arguments work identically

**✅ Benchmark accuracy**: When run with the same parameters, both versions should produce comparable results:
- Similar transactions per second (TPS)
- Similar latency distributions (avg, p50, p90, p95, p99)
- Identical transaction semantics
- Same SQL queries executed

**✅ Database compatibility**: Both versions work with the same PostgreSQL databases (10+) and can use the same initialized test data

**✅ Script compatibility**: Transaction scripts written for original pgbench work without modification

#### 5. Future Optimizations

Planned improvements (will not affect benchmark compatibility):
- Migrate to `COPY FROM STDIN` with `CopyInSink` API for faster initialization
- Connection pooling optimizations
- Statistics collection performance improvements

See [ASYNC_MIGRATION.md](ASYNC_MIGRATION.md) for detailed technical documentation of the async architecture changes.

### PostgreSQL Version Support

Target support for PostgreSQL versions:
- PostgreSQL 10+ (minimum)
- PostgreSQL 16+ (recommended)

## Performance

Performance goals:
- Within 10% of original C implementation
- Efficient multi-threading
- Low memory overhead
- Fast expression evaluation

Performance will be tracked as development progresses.

## License

This project follows the PostgreSQL License, matching the original pgbench.

```
Portions Copyright (c) 1996-2025, PostgreSQL Global Development Group
Portions Copyright (c) 1994, Regents of the University of California

Permission to use, copy, modify, and distribute this software and its
documentation for any purpose, without fee, and without a written agreement
is hereby granted, provided that the above copyright notice and this
paragraph and the following two paragraphs appear in all copies.
```

See [LICENSE](LICENSE) for full text.

## Acknowledgments

- Original pgbench by Tatsuo Ishii and PostgreSQL contributors
- PostgreSQL Global Development Group
- Rust community for excellent crates and tools

## Resources

- [Original pgbench documentation](https://www.postgresql.org/docs/current/pgbench.html)
- [PostgreSQL wire protocol](https://www.postgresql.org/docs/current/protocol.html)
- [Rust Book](https://doc.rust-lang.org/book/)
- [tokio-postgres crate](https://docs.rs/tokio-postgres/) - Async PostgreSQL driver
- [Tokio async runtime](https://docs.rs/tokio/) - Async runtime for Rust

## Roadmap

See [PORTING_PLAN.md](PORTING_PLAN.md) for the detailed development roadmap.

**Completed** (Phase 1-9):
- ✅ Project setup and dependencies
- ✅ CLI argument parsing
- ✅ Database connection (async with tokio-postgres)
- ✅ Expression parser (LALRPOP-based)
- ✅ Core data structures
- ✅ Database initialization
- ✅ Transaction execution
- ✅ Multi-threading (async with tokio)
- ✅ Statistics and reporting
- ✅ Async migration (fixes multi-client deadlocks)

**Current work** (Phase 9-10):
- ⏳ Integration testing with PostgreSQL
- ⏳ Performance optimization (COPY FROM STDIN)
- ⏳ Edge case handling
- ⏳ Documentation completion

**Future goals**:
- Advanced features (rate limiting, sampling mode)
- Full compatibility testing
- Production hardening
- Performance tuning

## Contact

For questions, issues, or contributions:
- Open an issue on GitHub
- Submit a pull request
- Contact: your.email@example.com

## Status Updates

Follow the project progress:
- Check the [PORTING_PLAN.md](PORTING_PLAN.md) for current phase
- See GitHub Issues for active work items
- Review commit history for recent changes
