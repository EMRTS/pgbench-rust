# Getting Started with pgbench-rust

## Overview

You now have a complete Rust project structure ready for porting pgbench from C to Rust. This document will help you get started with development.

## Project Status

**Phase**: Initial Setup Complete ✅

The project structure is in place with:
- Complete module skeleton
- All dependencies configured
- Comprehensive documentation
- Original C source files for reference
- Compiling project (with stub implementations)

## Quick Start

### 1. Verify Setup

```bash
cd pgbench-rust

# Check that the project compiles
cargo check

# Run tests (mostly stubs for now)
cargo test

# See available options (will panic with todo!() for now)
cargo run -- --help
```

### 2. Read the Documentation

Before coding, familiarize yourself with:

1. **[PORTING_PLAN.md](PORTING_PLAN.md)** - Complete roadmap with 10 phases
2. **[DEPENDENCIES.md](DEPENDENCIES.md)** - Mapping of C dependencies to Rust
3. **[ARCHITECTURE.md](ARCHITECTURE.md)** - Design decisions and architecture
4. **[README.md](README.md)** - Project overview and usage

### 3. Study the Original Source

The original C source files are in `original-source/`:

```bash
# Main implementation (8,015 lines)
less original-source/pgbench.c

# Type definitions
less original-source/pgbench.h

# Expression parser grammar
less original-source/exprparse.y

# Expression scanner
less original-source/exprscan.l
```

## Development Workflow

### Recommended Order

Follow the phases in [PORTING_PLAN.md](PORTING_PLAN.md):

**Phase 1: Foundation (Start Here)**
1. Complete CLI argument parsing in `src/cli.rs`
2. Implement database connection in `src/db/connection.rs`
3. Add basic logging throughout

**Phase 2: Core Data Structures**
1. Review and enhance `src/types.rs`
2. Ensure all C structs are represented

**Phase 3: Expression Parser**
1. Choose parser: pest (recommended), nom, or lalrpop
2. Port grammar from `exprparse.y`
3. Implement in `src/expr/parser.rs`
4. Test thoroughly

**Phase 4: Database Operations**
1. Implement initialization in `src/db/init.rs`
2. Add query execution in `src/db/query.rs`

**Phase 5: Random Number Generation**
1. Port PRNG in `src/random/prng.rs`
2. Implement distributions in `src/random/distributions.rs`

**Phase 6-10: Continue as per plan**

### Development Commands

```bash
# Fast checking during development
cargo check

# Run with detailed output
RUST_LOG=debug cargo run -- [args]

# Run tests
cargo test

# Run specific test
cargo test test_name

# Format code
cargo fmt

# Lint code
cargo clippy

# Build optimized release
cargo build --release

# Run benchmarks (when implemented)
cargo bench
```

### Adding a New Feature

Example: Implementing the expression parser

1. **Read the original C code**:
   ```bash
   less original-source/exprparse.y
   less original-source/exprscan.l
   ```

2. **Create pest grammar** (if using pest):
   ```bash
   # Create grammar file
   touch src/expr/grammar.pest
   ```

3. **Implement parser**:
   ```rust
   // src/expr/parser.rs
   use pest::Parser;
   use pest_derive::Parser;

   #[derive(Parser)]
   #[grammar = "expr/grammar.pest"]
   pub struct ExprParser;

   pub fn parse_expression(input: &str) -> PgBenchResult<PgBenchExpr> {
       // Implementation here
   }
   ```

4. **Write tests**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_parse_integer() {
           let expr = parse_expression("42").unwrap();
           // Assert expression is correct
       }
   }
   ```

5. **Test against original**:
   ```bash
   # Compare output with original pgbench
   ./test_compatibility.sh
   ```

## Key Design Decisions

### Why These Crates?

- **clap**: Industry standard for CLI parsing, derives reduce boilerplate
- **postgres**: Mature, synchronous (simpler than async for initial port)
- **pest**: PEG parser, easier to maintain than hand-written parsers
- **rand**: Standard RNG infrastructure, we implement Xoroshiro128** on top
- **thiserror**: Ergonomic error handling with good messages

### Synchronous vs Async

**Decision**: Start with synchronous I/O (postgres crate)

**Rationale**:
- Matches original C implementation
- Simpler to reason about
- Can migrate to async later if needed
- Multi-threading via std::thread (like C)

### Expression Parser Choice

**Recommendation**: Use pest

**Alternatives**:
1. **pest** (recommended)
   - Pro: Easy to read/maintain, good errors
   - Con: Slightly slower than hand-written

2. **nom**
   - Pro: Fast, more control
   - Con: More verbose, steeper learning curve

3. **lalrpop**
   - Pro: Most similar to Bison
   - Con: Build-time generation, complex

### Error Handling

Use `Result<T, PgBenchError>` everywhere:

```rust
// Good
pub fn do_something() -> PgBenchResult<Value> {
    let conn = connect_db()?;
    let result = execute_query(&conn)?;
    Ok(result)
}

// Avoid
pub fn do_something() -> Value {
    // Using unwrap() or panic!() in library code
    connect_db().unwrap()  // ❌ Don't do this
}
```

## Testing Strategy

### Unit Tests

Each module should have tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

### Integration Tests

In `tests/integration_test.rs`:

```rust
#[test]
#[ignore]  // Requires PostgreSQL
fn test_full_benchmark() {
    // End-to-end test
}
```

Run ignored tests:
```bash
cargo test -- --ignored
```

### Compatibility Tests

Compare with original pgbench:

```bash
# Run original pgbench
/path/to/postgres/src/bin/pgbench/pgbench -i -s 1 postgres://...

# Run rust version
cargo run -- -i -s 1 postgres://...

# Compare outputs
```

## Common Issues

### Issue: Can't connect to PostgreSQL

```bash
# Start PostgreSQL if not running
pg_ctl start

# Create test database
createdb pgbench_test

# Use connection string
export DATABASE_URL="postgres://user:pass@localhost/pgbench_test"
```

### Issue: Compilation errors

```bash
# Clear build artifacts
cargo clean

# Update dependencies
cargo update

# Check with verbose output
cargo check --verbose
```

### Issue: Tests fail

```bash
# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture

# Check test with debug logging
RUST_LOG=debug cargo test
```

## Next Steps

1. **Pick a starting module** from Phase 1 of the porting plan
2. **Study the C implementation** in `original-source/`
3. **Implement in Rust** following the architecture
4. **Write tests** to verify correctness
5. **Compare with original** for compatibility
6. **Document** any design decisions or deviations

## Getting Help

### Resources

- [PostgreSQL pgbench docs](https://www.postgresql.org/docs/current/pgbench.html)
- [Rust Book](https://doc.rust-lang.org/book/)
- [postgres crate docs](https://docs.rs/postgres/)
- [pest book](https://pest.rs/book/)
- [clap documentation](https://docs.rs/clap/)

### Code References

When you have questions about the C implementation:

1. Check `original-source/pgbench.c` for the function
2. Look up related PostgreSQL headers in the main source tree
3. Consult [DEPENDENCIES.md](DEPENDENCIES.md) for Rust equivalents

### Example: Finding How Something Works

**Question**: How does pgbench generate Gaussian random numbers?

**Steps**:
1. Search in `original-source/pgbench.c`:
   ```bash
   grep -n "gaussian" original-source/pgbench.c
   ```

2. Find the implementation (around line 1200-1250)

3. Check [DEPENDENCIES.md](DEPENDENCIES.md) for Rust approach:
   - Section "7. common/pg_prng.h"
   - Recommends using `rand_distr` crate

4. Implement in `src/random/distributions.rs`:
   ```rust
   use rand_distr::Normal;

   pub fn random_gaussian(...) -> PgBenchResult<i64> {
       let normal = Normal::new(mean, stddev)?;
       Ok(normal.sample(rng).round() as i64)
   }
   ```

## Project Structure Reference

```
pgbench-rust/
├── Cargo.toml              # Dependencies and configuration
├── README.md               # Project overview
├── PORTING_PLAN.md         # 11-week development roadmap
├── DEPENDENCIES.md         # C to Rust dependency mapping
├── ARCHITECTURE.md         # Design decisions
├── GETTING_STARTED.md      # This file
├── LICENSE                 # PostgreSQL license
├── .gitignore              # Git ignore rules
│
├── original-source/        # Reference C implementation
│   ├── pgbench.c          # Main implementation (8,015 lines)
│   ├── pgbench.h          # Type definitions
│   ├── exprparse.y        # Bison parser grammar
│   └── exprscan.l         # Flex scanner
│
├── src/                    # Rust source code
│   ├── main.rs            # Entry point
│   ├── lib.rs             # Library root
│   ├── cli.rs             # CLI parsing
│   ├── types.rs           # Core types
│   ├── error.rs           # Error types
│   ├── utils.rs           # Utilities
│   │
│   ├── db/                # Database operations
│   │   ├── mod.rs
│   │   ├── connection.rs  # Connection management
│   │   ├── init.rs        # Database initialization
│   │   └── query.rs       # Query execution
│   │
│   ├── expr/              # Expression parser
│   │   ├── mod.rs
│   │   ├── parser.rs      # Parse expressions
│   │   └── eval.rs        # Evaluate expressions
│   │
│   ├── script/            # Script handling
│   │   ├── mod.rs
│   │   ├── parser.rs      # Parse scripts
│   │   └── executor.rs    # Execute scripts
│   │
│   ├── random/            # RNG and distributions
│   │   ├── mod.rs
│   │   ├── prng.rs        # PRNG implementation
│   │   └── distributions.rs  # Statistical distributions
│   │
│   ├── worker/            # Multi-threading
│   │   ├── mod.rs
│   │   ├── thread.rs      # Thread management
│   │   └── state.rs       # Thread state
│   │
│   └── stats/             # Statistics
│       ├── mod.rs
│       ├── collector.rs   # Collect statistics
│       └── reporter.rs    # Report results
│
├── tests/                 # Integration tests
│   └── integration_test.rs
│
└── benches/               # Benchmarks (for benchmarking the benchmarker!)
```

## Milestones

Track your progress against these milestones:

- [ ] **Week 2**: Basic CLI and database connection working
- [ ] **Week 4**: Expression parser complete
- [ ] **Week 6**: Database operations functional
- [ ] **Week 8**: Multi-threading working
- [ ] **Week 10**: Feature complete
- [ ] **Week 11**: Production ready

Update the checkboxes as you complete each milestone!

## Contributing

If multiple people are working on this:

1. **Coordinate**: Pick different modules to avoid conflicts
2. **Branch**: Create feature branches for each module
3. **Test**: Ensure all tests pass before merging
4. **Document**: Update docs with any design decisions
5. **Review**: Have someone review your code

Good luck with the port! 🦀
