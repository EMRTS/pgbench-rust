# pgbench Port to Rust - Porting Plan

## Overview
This document outlines the plan to port PostgreSQL's pgbench benchmarking tool from C to Rust. pgbench is a simple benchmark program that runs repeated transactions against a PostgreSQL database and measures the performance.

## Source Analysis

### Original Source Structure
- **Location**: `src/bin/pgbench/` in PostgreSQL source tree
- **Main file**: `pgbench.c` (~8,015 lines)
- **Header**: `pgbench.h`
- **Parser**: `exprparse.y` (Bison grammar for expression parsing)
- **Scanner**: `exprscan.l` (Flex lexer for expression scanning)
- **Total complexity**: ~10,000 lines of C code

### Core Functionality
1. **Database Initialization**: Creates and populates benchmark tables
2. **Transaction Execution**: Runs customizable SQL transactions
3. **Expression Language**: Built-in expression parser for dynamic values
4. **Multi-threading**: Supports parallel client connections
5. **Statistics Collection**: Detailed performance metrics and reporting
6. **Custom Scripts**: Supports user-defined transaction scripts

## Dependencies to Port or Replace

### Critical PostgreSQL Libraries
1. **libpq** (PostgreSQL client library)
   - Rust replacement: `tokio-postgres` or `postgres` crate
   - Provides: Database connection, query execution, prepared statements

2. **libpgfeutils** (Frontend utilities)
   - Needs partial porting for specific utilities used by pgbench
   - See DEPENDENCIES.md for details

3. **libpgport** (Portability layer)
   - Most functionality available in Rust standard library or crates

### Specific Modules to Port

#### High Priority (Core Functionality)
1. **common/pg_prng.h**: Pseudo-random number generator
   - Used for: Random value generation in transactions
   - Rust approach: Implement using `rand` crate with compatible algorithms

2. **portability/instr_time.h**: High-precision time measurement
   - Used for: Performance timing and statistics
   - Rust approach: Use `std::time::Instant` and platform-specific APIs

3. **Expression Parser** (exprparse.y + exprscan.l)
   - Used for: Variable substitution and expression evaluation
   - Rust approach: Use `pest` or `nom` parser combinator, or port grammar

4. **common/int.h**: Safe integer arithmetic
   - Used for: Overflow checking in calculations
   - Rust approach: Native Rust checked arithmetic

#### Medium Priority (Utility Functions)
5. **common/logging.h**: Logging and error reporting
   - Rust approach: `log` + `env_logger` or `tracing` crate

6. **common/string.h**: String manipulation utilities
   - Rust approach: Standard library String methods

7. **fe_utils/option_utils.h**: Command-line option parsing
   - Rust approach: `clap` crate

8. **fe_utils/string_utils.h**: Additional string utilities
   - Rust approach: Standard library + custom implementations

#### Lower Priority (Can be simplified)
9. **common/username.h**: Username detection
   - Rust approach: `whoami` crate or standard library

10. **fe_utils/cancel.h**: Signal handling for Ctrl+C
    - Rust approach: `ctrlc` crate or signal-hook

11. **port/pg_bitutils.h**: Bit manipulation utilities
    - Rust approach: Standard library bit operations

12. **fe_utils/conditional.h**: Conditional compilation support
    - Rust approach: cfg attributes and feature flags

## Phased Porting Strategy

### Phase 1: Project Setup and Foundation (Week 1-2)
- [x] Set up Rust project structure with Cargo
- [ ] Create module structure matching C organization
- [ ] Set up basic CLI argument parsing with `clap`
- [ ] Establish database connection with `postgres` or `tokio-postgres`
- [ ] Port basic logging infrastructure
- [ ] Create unit test framework

**Deliverables**:
- Runnable Rust binary that can connect to PostgreSQL
- Basic command-line interface
- Test harness in place

### Phase 2: Core Data Structures (Week 2-3)
- [ ] Port PgBenchValue, PgBenchExpr types
- [ ] Port transaction command structures
- [ ] Port client state and thread structures
- [ ] Port statistics structures (StatsData, etc.)
- [ ] Implement safe memory management (leverage Rust ownership)

**Deliverables**:
- All core data structures defined in Rust
- Basic type conversions working
- Memory safety verified by compiler

### Phase 3: Expression Parser (Week 3-4)
- [ ] Choose parsing approach (pest, nom, or manual)
- [ ] Port expression grammar from exprparse.y
- [ ] Port scanner logic from exprscan.l
- [ ] Implement expression evaluation engine
- [ ] Port all built-in functions (math, random, hash, etc.)
- [ ] Write comprehensive expression tests

**Deliverables**:
- Working expression parser and evaluator
- Support for all pgbench expression features
- Test coverage for expression language

### Phase 4: Database Operations (Week 4-5)
- [ ] Port database initialization logic (-i mode)
- [ ] Port table creation and population
- [ ] Port transaction script parsing
- [ ] Implement prepared statement handling
- [ ] Port custom script file loading
- [ ] Implement meta-commands (\set, \sleep, etc.)

**Deliverables**:
- Database initialization working
- Script parsing complete
- Basic transaction execution

### Phase 5: Random Number Generation (Week 5-6)
- [ ] Port pg_prng implementation or use equivalent
- [ ] Implement uniform distribution
- [ ] Implement Gaussian distribution
- [ ] Implement exponential distribution
- [ ] Implement Zipfian distribution
- [ ] Verify statistical properties match original

**Deliverables**:
- All random distributions working
- Statistical tests passing
- Compatible with original pgbench seeds

### Phase 6: Multi-threading and Connection Management (Week 6-7)
- [ ] Design thread-safe architecture
- [ ] Implement worker thread pool
- [ ] Port thread barrier synchronization
- [ ] Implement connection pooling per thread
- [ ] Port socket polling (select/poll)
- [ ] Handle connection errors and retries

**Deliverables**:
- Multi-threaded execution working
- Thread synchronization correct
- Connection management robust

### Phase 7: Statistics and Reporting (Week 7-8)
- [ ] Port statistics collection (TPS, latency, etc.)
- [ ] Implement per-thread statistics
- [ ] Port aggregation logic
- [ ] Port progress reporting
- [ ] Port final results reporting
- [ ] Port log file generation (--log)
- [ ] Implement per-transaction logging

**Deliverables**:
- Complete statistics collection
- Accurate performance metrics
- All reporting modes working

### Phase 8: Advanced Features (Week 8-9)
- [ ] Port rate limiting (--rate)
- [ ] Port latency limit (--latency-limit)
- [ ] Port connection retry logic
- [ ] Port vacuum and checkpoint options
- [ ] Port partition support
- [ ] Port unlogged tables option
- [ ] Port sampling (--sampling-rate)

**Deliverables**:
- All advanced features implemented
- Feature parity with C version

### Phase 9: Testing and Validation (Week 9-10)
- [ ] Create comprehensive integration tests
- [ ] Test against actual PostgreSQL instances
- [ ] Compare results with original pgbench
- [ ] Performance benchmarking
- [ ] Edge case testing
- [ ] Error handling validation
- [ ] Platform testing (Linux, macOS, Windows)

**Deliverables**:
- Full test coverage
- Validation against C version
- Performance analysis

### Phase 10: Polish and Documentation (Week 10-11)
- [ ] Code cleanup and refactoring
- [ ] Add inline documentation
- [ ] Write user documentation
- [ ] Create migration guide from C pgbench
- [ ] Add usage examples
- [ ] Performance optimization
- [ ] Final code review

**Deliverables**:
- Production-ready code
- Complete documentation
- Migration guide

## Technical Decisions

### Rust Crates to Use
- **postgres** or **tokio-postgres**: PostgreSQL client (choose based on async needs)
- **clap** v4: Command-line argument parsing
- **rand** + **rand_distr**: Random number generation
- **log** + **env_logger**: Logging infrastructure
- **pest** or **nom**: Parser (for expression language)
- **rayon** or native threads: Parallelism
- **anyhow** or **thiserror**: Error handling
- **serde**: Serialization (if needed for config)

### Architecture Decisions
1. **Synchronous vs Async**: Start with synchronous I/O (using `postgres` crate) to match C behavior; can add async later if needed
2. **Threading Model**: Native threads (std::thread) to match C's pthread model
3. **Expression Parser**: Use `pest` for maintainability, or `nom` for performance
4. **Error Handling**: Use Result<T, E> with custom error types
5. **Memory Management**: Leverage Rust's ownership system; minimize Arc/Mutex where possible

### Compatibility Goals
1. **Command-line Interface**: 100% compatible with original pgbench flags
2. **Script Format**: Support all existing pgbench scripts
3. **Expression Language**: Full compatibility with expression syntax
4. **Output Format**: Match output format for easy comparison
5. **Random Seed**: Same results given same seed (important for reproducibility)

## Testing Strategy
1. **Unit Tests**: For each module and function
2. **Integration Tests**: End-to-end scenarios with real database
3. **Compatibility Tests**: Compare output with original pgbench
4. **Performance Tests**: Ensure no significant performance regression
5. **Stress Tests**: Large scale factors, many threads, long duration

## Success Criteria
- [ ] All pgbench command-line options supported
- [ ] All built-in scripts work correctly
- [ ] Custom scripts load and execute
- [ ] Expression language fully functional
- [ ] Statistics match original pgbench
- [ ] Performance within 10% of C version
- [ ] No memory leaks or safety issues
- [ ] Cross-platform support (Linux, macOS, Windows)
- [ ] Comprehensive documentation

## Risks and Mitigation
1. **Expression Parser Complexity**:
   - Risk: Hard to port Bison/Flex to Rust
   - Mitigation: Use well-tested parser library (pest/nom)

2. **Performance Parity**:
   - Risk: Rust version slower than C
   - Mitigation: Profile early, optimize hot paths, use same algorithms

3. **PostgreSQL Protocol Nuances**:
   - Risk: Subtle differences in libpq behavior
   - Mitigation: Thorough testing with various PostgreSQL versions

4. **Platform-specific Code**:
   - Risk: Windows threading, socket handling differences
   - Mitigation: Use cross-platform crates, test on all platforms

5. **Random Distribution Accuracy**:
   - Risk: Statistical distributions don't match exactly
   - Mitigation: Port exact algorithms from C, validate statistically

## Timeline
**Estimated Duration**: 11 weeks (can be parallelized with multiple developers)

**Milestones**:
- Week 2: Basic runnable tool
- Week 4: Expression parser working
- Week 6: Database operations complete
- Week 8: Multi-threading functional
- Week 10: Feature complete
- Week 11: Production ready

## Next Steps
1. Review and approve this plan
2. Set up development environment
3. Begin Phase 1: Project setup
4. Establish CI/CD pipeline
5. Create GitHub repository structure
