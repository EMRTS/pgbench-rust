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

**Current Phase**: Initial setup and planning
**Version**: 0.1.0 (Pre-alpha)

See [PORTING_PLAN.md](PORTING_PLAN.md) for the detailed porting roadmap.

## Features

The goal is to support all features of the original pgbench:

### Core Functionality
- [x] Project structure and dependencies
- [ ] Database initialization with benchmarking tables
- [ ] Built-in TPC-B-like transaction scenarios
- [ ] Custom transaction scripts
- [ ] Multiple concurrent client connections
- [ ] Threaded execution for parallel workload

### Benchmarking Options
- [ ] Duration-based testing (-T)
- [ ] Transaction count-based testing (-t)
- [ ] Multiple clients (-c)
- [ ] Multiple threads (-j)
- [ ] Rate limiting (--rate)
- [ ] Latency limit (--latency-limit)

### Expression Language
- [ ] Variable substitution (:varname)
- [ ] Arithmetic expressions
- [ ] Random number generation (uniform, gaussian, exponential, zipfian)
- [ ] Hash functions (FNV-1a, MurmurHash2)
- [ ] Built-in functions (abs, sqrt, pow, etc.)
- [ ] CASE expressions

### Reporting
- [ ] Transaction per second (TPS)
- [ ] Latency statistics (average, median, p90, p95, p99)
- [ ] Per-transaction logging
- [ ] Progress reporting
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

Currently in development. Any intentional differences will be documented here.

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
- [postgres crate](https://docs.rs/postgres/)

## Roadmap

See [PORTING_PLAN.md](PORTING_PLAN.md) for the detailed development roadmap.

**Short-term goals** (Phase 1-3):
- ✅ Project setup
- ⏳ Basic CLI and database connection
- ⏳ Expression parser implementation
- ⏳ Core data structures

**Medium-term goals** (Phase 4-6):
- Database initialization
- Transaction execution
- Multi-threading

**Long-term goals** (Phase 7-10):
- Statistics and reporting
- Advanced features
- Full compatibility
- Production readiness

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
