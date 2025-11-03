# pgbench Dependencies Documentation

This document details all PostgreSQL dependencies used by pgbench and how they should be handled in the Rust port.

## Overview
pgbench depends on several PostgreSQL internal libraries and modules. This document maps each dependency to its purpose and recommended Rust replacement.

## PostgreSQL Libraries

### 1. libpq (PostgreSQL Client Library)
**C Header**: `libpq-fe.h`
**Purpose**: Main PostgreSQL client library for database connectivity
**Usage in pgbench**:
- Database connections (PQconnectdb, PQfinish)
- Query execution (PQexec, PQexecPrepared)
- Prepared statements (PQprepare)
- Result handling (PQgetvalue, PQntuples, PQclear)
- Error handling (PQerrorMessage, PQstatus)
- Async operations (PQsendQuery, PQgetResult)

**Rust Replacement**: `postgres` or `tokio-postgres` crate
- **postgres**: Synchronous, simpler, good starting point
- **tokio-postgres**: Async, better performance for high concurrency

**Key Functions to Replace**:
```c
PGconn *PQconnectdb(const char *conninfo)          → postgres::Client::connect()
PGresult *PQexec(PGconn *conn, const char *query)  → client.execute() / client.query()
void PQfinish(PGconn *conn)                        → Drop trait (automatic)
char *PQerrorMessage(const PGconn *conn)           → Error handling via Result<>
```

### 2. libpgfeutils (Frontend Utilities)
**Purpose**: Common utilities for PostgreSQL frontend tools

#### Modules Used:

##### cancel.h
**Purpose**: Signal handling for query cancellation (Ctrl+C)
**Functions**:
- `setup_cancel_handler()`: Set up signal handlers
- `cancel_connection()`: Send cancel request to server

**Rust Replacement**: `ctrlc` crate + postgres cancel token
```rust
use ctrlc;
// Set up handler that calls client.cancel_query()
```

##### conditional.h
**Purpose**: Conditional execution in scripts (\\if, \\elif, \\else, \\endif)
**Functions**: Stack-based conditional processing

**Rust Replacement**: Custom implementation with enum-based state machine

##### option_utils.h
**Purpose**: Common command-line option handling
**Functions**: Standard option parsing utilities

**Rust Replacement**: `clap` v4 with derive macros
```rust
use clap::Parser;
#[derive(Parser)]
struct Args { ... }
```

##### string_utils.h
**Purpose**: String manipulation utilities
**Functions**:
- `pg_strdup()`: Safe string duplication
- `psprintf()`: Formatted string allocation

**Rust Replacement**: Rust String, format!(), and std methods

##### psqlscan.h
**Purpose**: SQL scanner for parsing scripts
**Functions**: Tokenize SQL commands, handle psql meta-commands

**Rust Replacement**: Custom lexer or `logos` crate for tokenization

### 3. libpgport (Portability Library)
**Purpose**: Platform abstraction layer

#### Modules Used:

##### pg_pthread.h
**Purpose**: Cross-platform pthread wrapper
**Functions**: Thread creation, barriers, mutexes

**Rust Replacement**: `std::thread` and `std::sync` modules
```rust
use std::thread;
use std::sync::{Arc, Mutex, Barrier};
```

##### pg_bitutils.h
**Purpose**: Bit manipulation utilities
**Functions**:
- `pg_leftmost_one_pos32()`: Find highest set bit
- `pg_rightmost_one_pos32()`: Find lowest set bit

**Rust Replacement**: Native Rust methods
```rust
u32::leading_zeros()
u32::trailing_zeros()
```

## PostgreSQL Common Modules

### 4. common/int.h
**Purpose**: Safe integer arithmetic with overflow checking
**Functions**:
- `pg_add_s64_overflow()`: Safe 64-bit addition
- `pg_mul_s64_overflow()`: Safe 64-bit multiplication

**Rust Replacement**: Native checked arithmetic
```rust
i64::checked_add()
i64::checked_mul()
i64::saturating_add()
```

### 5. common/logging.h
**Purpose**: Logging and error reporting
**Functions**:
- `pg_log_error()`: Error messages
- `pg_log_warning()`: Warning messages
- `pg_log_info()`: Info messages
- `pg_fatal()`: Fatal errors with exit

**Rust Replacement**: `log` crate + `env_logger`
```rust
use log::{error, warn, info};
error!("message");
warn!("message");
info!("message");
```

### 6. common/pg_prng.h
**Purpose**: Pseudo-random number generator (Xoroshiro128**)
**Functions**:
- `pg_prng_seed()`: Seed generator
- `pg_prng_uint64()`: Generate random uint64
- `pg_prng_double()`: Generate random double [0.0, 1.0)

**Rust Replacement**: Custom implementation using `rand` crate
```rust
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

// Need to implement Xoroshiro128** for compatibility
// Or verify rand::rngs::SmallRng provides equivalent distribution
```

**IMPORTANT**: Must match exact algorithm for reproducible benchmarks!

### 7. common/string.h
**Purpose**: String utilities
**Functions**:
- `pg_strdup()`: Allocate and copy string
- `pg_malloc()`: Checked memory allocation

**Rust Replacement**: Standard library
```rust
String::from()
str.to_string()
String::clone()
```

### 8. common/username.h
**Purpose**: Get current username
**Functions**: `get_user_name()`

**Rust Replacement**: `whoami` crate
```rust
use whoami;
let username = whoami::username();
```

### 9. portability/instr_time.h
**Purpose**: High-resolution time measurement
**Type**: `instr_time` struct
**Functions**:
- `INSTR_TIME_SET_CURRENT()`: Get current time
- `INSTR_TIME_GET_MICROSEC()`: Convert to microseconds
- `INSTR_TIME_SUBTRACT()`: Calculate difference

**Rust Replacement**: `std::time::Instant`
```rust
use std::time::Instant;
let start = Instant::now();
let duration = start.elapsed();
let micros = duration.as_micros();
```

### 10. catalog/pg_class_d.h
**Purpose**: PostgreSQL catalog constants
**Usage**: Table relkind constants (RELKIND_RELATION, etc.)

**Rust Replacement**: Define as Rust constants
```rust
const RELKIND_RELATION: char = 'r';
const RELKIND_INDEX: char = 'i';
```

## Expression Parser Dependencies

### 11. exprparse.y (Bison Grammar)
**Purpose**: Expression language parser
**Size**: ~1,000 lines of Bison grammar
**Features**:
- Arithmetic operators (+, -, *, /, %)
- Comparison operators (=, <, >, <=, >=, !=)
- Logical operators (AND, OR, NOT)
- Bitwise operators (&, |, #, ~, <<, >>)
- Functions (sqrt, abs, random, hash, etc.)
- Variables (:varname)
- CASE expressions

**Rust Replacement Options**:

**Option A: pest** (Recommended for maintainability)
```rust
// Define grammar in PEG syntax
// Clean, declarative, good error messages
```

**Option B: nom** (Better performance)
```rust
// Parser combinators in Rust code
// Faster parsing, more control
```

**Option C: lalrpop** (Most similar to Bison)
```rust
// LR parser generator
// Closest to original grammar
```

### 12. exprscan.l (Flex Scanner)
**Purpose**: Lexical scanner for expressions
**Features**:
- Token recognition
- String literals
- Number parsing (int, float)
- Variable names
- Operators

**Rust Replacement**:
- Integrated with parser (pest/nom handle lexing)
- Or standalone lexer using `logos` crate

## Platform Dependencies

### Standard C Library
The following C standard library features are used:

**Math Functions** (math.h):
- `sqrt()`, `log()`, `exp()`, `pow()`, `fabs()`: Use `f64` methods in Rust
- `M_PI`: `std::f64::consts::PI`

**Time Functions** (time.h, sys/time.h):
- `gettimeofday()`: Use `std::time::SystemTime`
- `time()`: Use `std::time::SystemTime`

**Signal Handling** (signal.h):
- `SIGINT`: Use `ctrlc` crate
- `SIGTERM`: Use `signal-hook` crate

**Resource Limits** (sys/resource.h):
- `getrlimit()`: Check file descriptor limits
- Rust: Use `rlimit` crate or platform-specific code

**Thread Synchronization** (pthread.h):
- `pthread_barrier_t`: Use `std::sync::Barrier`
- `pthread_t`: Use `std::thread::JoinHandle`
- `pthread_mutex_t`: Use `std::sync::Mutex`

**Socket/Poll** (poll.h, sys/select.h):
- `ppoll()`, `select()`: For async query execution
- Rust: Use `mio` crate or `tokio` runtime

## Dependency Tree Summary

```
pgbench
├── libpq (Database connectivity)
│   └── Rust: postgres / tokio-postgres crate
├── Expression Parser
│   ├── exprparse.y (grammar)
│   │   └── Rust: pest / nom / lalrpop
│   └── exprscan.l (lexer)
│       └── Rust: integrated with parser
├── Random Number Generation
│   └── pg_prng (Xoroshiro128**)
│       └── Rust: custom impl using rand crate
├── Threading
│   ├── pthread (POSIX threads)
│   │   └── Rust: std::thread
│   └── pthread_barrier_t
│       └── Rust: std::sync::Barrier
├── Time Measurement
│   └── instr_time
│       └── Rust: std::time::Instant
├── Logging
│   └── common/logging
│       └── Rust: log + env_logger
├── Command-line Parsing
│   └── getopt_long / option_utils
│       └── Rust: clap v4
├── String Utilities
│   └── common/string
│       └── Rust: std::string
└── Platform Support
    ├── Signal handling → ctrlc crate
    ├── Username → whoami crate
    └── Bit operations → std methods
```

## Files to Reference During Porting

### Critical Source Files
Copy these to the Rust project as reference:
- `src/bin/pgbench/pgbench.c` - Main implementation (8,015 lines)
- `src/bin/pgbench/pgbench.h` - Type definitions
- `src/bin/pgbench/exprparse.y` - Expression grammar
- `src/bin/pgbench/exprscan.l` - Expression lexer

### Reference Header Files
Keep these for understanding data structures and algorithms:
- `src/include/common/pg_prng.h` - PRNG interface
- `src/common/pg_prng.c` - PRNG implementation
- `src/include/portability/instr_time.h` - Time measurement
- `src/include/common/int.h` - Integer operations
- `src/include/fe_utils/conditional.h` - Conditional logic

### Algorithm Implementations
Need to review and potentially port:
- Zipfian distribution algorithm
- Gaussian distribution (Box-Muller)
- FNV-1a hash implementation
- MurmurHash2 implementation
- Permutation algorithm

## Testing Requirements

### Compatibility Tests
For each dependency replacement, verify:
1. **Functional equivalence**: Same behavior as C version
2. **Performance**: No significant regression
3. **Correctness**: Statistical tests for RNG, timing accuracy
4. **Edge cases**: Error handling, boundary conditions

### Critical Compatibility Points
- **RNG with same seed → same sequence** (for reproducibility)
- **Time measurements accurate to microseconds**
- **Expression parser accepts all valid pgbench expressions**
- **Database operations match libpq behavior**
- **Error messages clear and helpful**

## Rust Crate Dependencies (Cargo.toml)

```toml
[dependencies]
# Database connectivity
postgres = "0.19"  # Or tokio-postgres for async

# Command-line parsing
clap = { version = "4.5", features = ["derive"] }

# Random number generation
rand = "0.8"
rand_distr = "0.4"  # For Gaussian, Exponential distributions

# Logging
log = "0.4"
env_logger = "0.11"

# Error handling
anyhow = "1.0"  # Or thiserror

# Parsing (choose one)
pest = "2.7"
pest_derive = "2.7"
# OR
# nom = "7.1"
# OR
# lalrpop = "0.20"
# lalrpop-util = "0.20"

# Cross-platform utilities
ctrlc = "3.4"           # Signal handling
whoami = "1.5"          # Username
signal-hook = "0.3"     # Advanced signal handling

# Serialization (if needed)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[build-dependencies]
# If using lalrpop
# lalrpop = "0.20"
```

## Migration Priority

### Phase 1 (Essential)
1. libpq → postgres
2. option_utils → clap
3. logging → log/env_logger
4. instr_time → std::time::Instant

### Phase 2 (Core Logic)
5. Expression parser (exprparse.y/exprscan.l) → pest/nom
6. pg_prng → custom implementation
7. pthread → std::thread
8. int.h → checked arithmetic

### Phase 3 (Polish)
9. cancel → ctrlc
10. username → whoami
11. string utilities → std
12. conditional → custom

## Notes for Developers

1. **Start with libpq replacement**: Get database connectivity working first
2. **Expression parser is critical**: Many features depend on it
3. **RNG must be exact**: Reproducibility is important for benchmarking
4. **Time measurement must be accurate**: This is a benchmarking tool
5. **Error messages should be helpful**: Users need clear feedback
6. **Test incrementally**: Verify each replacement before moving on

## Useful PostgreSQL Source Locations

For reference when porting:
- `src/common/pg_prng.c`: PRNG implementation
- `src/common/d2s.c`: Double-to-string conversion
- `src/port/pgcheckdir.c`: Directory checking
- `src/fe_utils/psqlscan.l`: SQL scanning (reference)
- `src/common/hashfn.c`: Hash functions

## External Documentation
- PostgreSQL libpq documentation: https://www.postgresql.org/docs/current/libpq.html
- pgbench documentation: https://www.postgresql.org/docs/current/pgbench.html
- Xoroshiro128** algorithm: https://prng.di.unimi.it/
