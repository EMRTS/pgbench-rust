# pgbench-rust Architecture

## Overview

This document describes the architecture and design decisions for pgbench-rust, a Rust port of PostgreSQL's pgbench benchmarking tool.

## Design Goals

1. **Full Compatibility**: 100% command-line and script compatibility with original pgbench
2. **Memory Safety**: Leverage Rust's ownership system to prevent memory errors
3. **Maintainability**: Clean, modular architecture that's easy to understand and extend
4. **Performance**: Match or exceed the performance of the C implementation
5. **Correctness**: Produce identical statistical results for reproducible benchmarks

## Module Organization

```
pgbench-rust/
├── src/
│   ├── main.rs           # Entry point and CLI coordination
│   ├── lib.rs            # Library root, re-exports
│   ├── cli.rs            # Command-line argument parsing (clap)
│   ├── types.rs          # Core data structures
│   ├── error.rs          # Error types
│   ├── utils.rs          # Utility functions
│   ├── db/               # Database operations
│   ├── expr/             # Expression parser and evaluator
│   ├── script/           # Script parsing and execution
│   ├── random/           # Random number generation
│   ├── worker/           # Thread management
│   └── stats/            # Statistics collection
```

## Core Components

### 1. CLI Module (`cli.rs`)

**Responsibility**: Parse and validate command-line arguments

**Key Types**:
- `Args`: Main argument structure using `clap` derive macros

**Design Decisions**:
- Use `clap` v4 for robust, idiomatic argument parsing
- Validation logic separate from parsing
- Help text matches original pgbench format

### 2. Types Module (`types.rs`)

**Responsibility**: Define core data structures

**Key Types**:
- `PgBenchValue`: Represents expression values (int, double, bool, null)
- `PgBenchExpr`: Expression AST nodes
- `PgBenchFunction`: Enumeration of built-in functions
- `TransactionStats`: Statistics accumulator
- `ThreadState`: Per-thread state
- `Command`: Script command (SQL or meta-command)

**Design Decisions**:
- Use Rust enums instead of C unions for type safety
- Derive traits where possible (Debug, Clone, PartialEq)
- Keep types simple and focused

### 3. Error Module (`error.rs`)

**Responsibility**: Define error types and conversions

**Key Types**:
- `PgBenchError`: Main error enum using `thiserror`
- `PgBenchResult<T>`: Type alias for Result

**Design Decisions**:
- Use `thiserror` for ergonomic error definitions
- Provide context in error messages (line numbers, etc.)
- Implement From traits for external error types

### 4. Database Module (`db/`)

**Responsibility**: Handle all database operations

**Submodules**:
- `connection.rs`: Connection management
- `init.rs`: Database initialization (create and populate tables)
- `query.rs`: Query execution utilities

**Key Types**:
- `PgBenchConnection`: Wraps `postgres::Client`

**Design Decisions**:
- Start with synchronous `postgres` crate (can migrate to async later)
- Connection pooling per thread
- Prepared statement caching
- Retry logic for transient failures

**Database Schema**:
```sql
-- Same as original pgbench
CREATE TABLE pgbench_accounts (
    aid     INT NOT NULL,
    bid     INT,
    abalance INT,
    filler  CHAR(84)
);

CREATE TABLE pgbench_branches (
    bid     INT NOT NULL,
    bbalance INT,
    filler  CHAR(88)
);

CREATE TABLE pgbench_tellers (
    tid     INT NOT NULL,
    bid     INT,
    tbalance INT,
    filler  CHAR(84)
);

CREATE TABLE pgbench_history (
    tid     INT,
    bid     INT,
    aid     INT,
    delta   INT,
    mtime   TIMESTAMP,
    filler  CHAR(22)
);
```

### 5. Expression Module (`expr/`)

**Responsibility**: Parse and evaluate pgbench expressions

**Submodules**:
- `parser.rs`: Parse expression strings into AST
- `eval.rs`: Evaluate expressions to produce values

**Key Types**:
- `EvalContext`: Variable bindings and state

**Design Decisions**:
- Use `pest` PEG parser for maintainability (can switch to `nom` if performance critical)
- Separate parsing from evaluation (classic two-phase approach)
- Support all original pgbench expression features

**Expression Grammar**:
```
expr ::= literal
       | variable
       | function '(' args ')'
       | expr binop expr
       | unop expr
       | '(' expr ')'
       | CASE when_list [ELSE expr] END

variable ::= ':' identifier

function ::= identifier

when_list ::= WHEN expr THEN expr [when_list]
```

### 6. Script Module (`script/`)

**Responsibility**: Parse and execute transaction scripts

**Submodules**:
- `parser.rs`: Parse script files
- `executor.rs`: Execute parsed scripts

**Key Types**:
- `Command`: SQL or meta-command
- `MetaCommand`: \set, \sleep, \if, etc.

**Design Decisions**:
- Line-by-line parser (simpler than full SQL parser)
- Support all meta-commands from original
- Conditional execution with stack-based state machine

**Meta-commands**:
- `\set variable expression`: Set variable
- `\setshell variable command`: Set variable from shell command
- `\sleep time`: Sleep for specified time
- `\if expression`: Conditional execution
- `\elif expression`: Else-if
- `\else`: Else
- `\endif`: End conditional
- `\startpipeline`: Begin pipeline mode
- `\endpipeline`: End pipeline mode

### 7. Random Module (`random/`)

**Responsibility**: Random number generation with multiple distributions

**Submodules**:
- `prng.rs`: Core PRNG (Xoroshiro128**)
- `distributions.rs`: Statistical distributions

**Key Types**:
- `PgBenchRng`: PRNG compatible with pg_prng

**Design Decisions**:
- Implement Xoroshiro128** for exact compatibility
- Use `rand` crate infrastructure (RngCore, SeedableRng)
- Use `rand_distr` for Gaussian and Exponential
- Custom implementation for Zipfian (most complex)

**Distributions**:
- **Uniform**: Simple modulo operation
- **Gaussian**: Box-Muller transform
- **Exponential**: Inverse transform sampling
- **Zipfian**: Rejection method (complex, see implementation notes)

### 8. Worker Module (`worker/`)

**Responsibility**: Multi-threaded workload execution

**Submodules**:
- `thread.rs`: Thread pool and coordination
- `state.rs`: Per-thread state management

**Key Types**:
- `ThreadState`: Per-thread data

**Design Decisions**:
- Use native threads (std::thread) to match C behavior
- Thread barrier for synchronized start (std::sync::Barrier)
- Lock-free per-thread statistics (aggregate at end)
- Graceful shutdown via atomic flags

**Threading Model**:
```
Main Thread
├── Initialize database (if -i)
├── Create N worker threads
├── Barrier: synchronize start
├── Wait for completion or timeout
├── Aggregate statistics
└── Report results

Worker Thread (×N)
├── Create database connections (C/N each)
├── Barrier: wait for all threads
├── Execute transactions
│   ├── Parse script
│   ├── Evaluate expressions
│   ├── Execute SQL
│   └── Record statistics
└── Return statistics
```

### 9. Statistics Module (`stats/`)

**Responsibility**: Collect and report performance metrics

**Submodules**:
- `collector.rs`: Statistics collection
- `reporter.rs`: Report formatting and output

**Key Types**:
- `StatsCollector`: Accumulates statistics
- `TransactionStats`: Raw statistics data

**Design Decisions**:
- Per-thread collectors (lock-free during collection)
- Aggregate at end for final report
- Support for latency percentiles (P50, P90, P95, P99)
- Optional per-transaction logging

**Statistics Collected**:
- Transaction count
- Total time
- TPS (transactions per second)
- Latency: min, max, average, stddev
- Percentiles: P50, P90, P95, P99
- Per-transaction type breakdown (if multiple scripts)

## Data Flow

### Initialization Mode (-i)

```
main()
  → parse_args()
  → connect_database()
  → initialize_database()
      ├── drop_tables()
      ├── create_tables()
      ├── populate_tables(scale_factor)
      ├── create_indexes()
      └── vacuum_analyze()
```

### Benchmark Mode (default)

```
main()
  → parse_args()
  → load_scripts()
  → create_worker_threads()
      └── worker_thread()
          ├── connect_database()
          ├── barrier_wait()  # synchronize start
          └── loop until done:
              ├── select_script()
              ├── execute_transaction()
              │   ├── parse_commands()
              │   ├── evaluate_expressions()
              │   ├── execute_sql()
              │   └── record_latency()
              └── check_limits()
  → aggregate_statistics()
  → report_results()
```

## Expression Evaluation

```
Input: ":scale * 100000 + random(1, 100)"

Parsing:
  → Tokenize: [":", "scale", "*", "100000", "+", "random", "(", "1", ",", "100", ")"]
  → Parse tree:
      Add
      ├── Mul
      │   ├── Variable("scale")
      │   └── Constant(100000)
      └── Function(Random, [Constant(1), Constant(100)])

Evaluation:
  → Resolve variables: scale = 10
  → Evaluate recursively:
      Add(Mul(10, 100000), Random(1, 100))
      = Add(1000000, Random(1, 100))
      = Add(1000000, 42)
      = 1000042
```

## Performance Considerations

### Hot Paths
1. **Transaction execution loop**: Most time spent here
2. **Expression evaluation**: Optimize for common cases
3. **Random number generation**: Should be fast
4. **Database I/O**: Inherent bottleneck

### Optimization Strategies
1. **Connection pooling**: Reuse connections
2. **Prepared statements**: Cache prepared statements
3. **Batch operations**: Where possible
4. **Lock-free statistics**: Per-thread, aggregate at end
5. **Inline small functions**: Let compiler optimize
6. **Avoid allocations**: In hot paths
7. **SIMD**: For hash functions if needed

### Memory Usage
- **Per thread**: Connection, RNG state, variables, statistics
- **Shared**: Script definitions, configuration
- **Total**: Should scale linearly with thread count

## Error Handling Strategy

### Error Categories
1. **Fatal errors**: Connection failure, script parse errors → exit
2. **Transient errors**: Serialization failures → retry
3. **Transaction errors**: Query failures → log and continue
4. **Validation errors**: Invalid arguments → report and exit

### Error Propagation
- Use `Result<T, PgBenchError>` throughout
- Use `?` operator for clean propagation
- Convert external errors with `From` trait
- Provide context in error messages

## Testing Strategy

### Unit Tests
- Each module has `#[cfg(test)]` section
- Test pure functions in isolation
- Mock database operations where needed

### Integration Tests
- Full end-to-end scenarios
- Requires running PostgreSQL instance
- Mark with `#[ignore]` if database required

### Property Tests
- Use `proptest` for RNG statistical properties
- Verify distributions are correct

### Compatibility Tests
- Compare output with original pgbench
- Same input → same output (with same seed)
- Verify TPS calculations match

## Future Enhancements

### Phase 1 (Core functionality)
- Basic benchmarking
- Expression language
- Multi-threading

### Phase 2 (Advanced features)
- Async I/O (migrate to tokio-postgres)
- Better progress reporting
- More detailed statistics

### Phase 3 (Extensions)
- Custom metrics collection
- JSON output format
- Web dashboard (optional)
- Distributed benchmarking (optional)

## References

- [Original pgbench documentation](https://www.postgresql.org/docs/current/pgbench.html)
- [PostgreSQL source: src/bin/pgbench/](https://github.com/postgres/postgres/tree/master/src/bin/pgbench)
- [Rust postgres crate](https://docs.rs/postgres/)
- [pest parser](https://pest.rs/)
- [Xoroshiro128** algorithm](https://prng.di.unimi.it/)
