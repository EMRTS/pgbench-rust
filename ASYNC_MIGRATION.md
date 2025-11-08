# Async Migration Progress (Phase 9.2)

**Goal**: Migrate from synchronous `postgres` crate to async `tokio-postgres` to fix multi-client single-thread deadlock issue.

**Status**: 🚧 IN PROGRESS (approx 40% complete)

---

## ✅ Completed

### 1. Dependencies (Cargo.toml)
- ✅ Replaced `postgres = "0.19"` with `tokio-postgres = "0.7"`
- ✅ Added `tokio = "1.40"` with full features
- ✅ Added `bytes = "1.7"` for CopyInSink

### 2. Connection Layer (src/db/connection.rs)
- ✅ Converted `PgBenchConnection::connect()` to async
- ✅ Converted `connect_with_retry()` to async
- ✅ Converted `validate_connection()` to async
- ✅ Converted `execute()`, `query()`, `is_alive()`, `reconnect()` to async
- ✅ Updated `copy_in()` to return `tokio_postgres::CopyInSink`
- ✅ Removed `CopyWriter` wrapper (obsolete with async)
- ✅ Updated all connection tests to use `#[tokio::test]`
- ✅ Spawned connection background task in `connect_with_retry()`
- ✅ Changed `std::thread::sleep` to `tokio::time::sleep`

### 3. Query Executor (src/db/query.rs)
- ✅ Converted `execute()` and `execute_with_params()` to async
- ✅ Converted `execute_simple()` to async
- ✅ Converted `execute_extended()` to async
- ✅ Converted `execute_prepared()` to async
- ✅ Converted `execute_update()` to async
- ✅ Updated all `postgres::types::ToSql` to `tokio_postgres::types::ToSql`
- ✅ Added `.await` to all `prepare_typed()` calls

### 4. Database Initialization (src/db/init.rs)
- ✅ Converted function signatures to async:
  - `initialize_database()`
  - `drop_tables()`
  - `create_tables()`
  - `create_partitions()`
  - `generate_data_client_side()`
  - `generate_data_server_side()`
  - `populate_table()`
  - `vacuum_tables()`
  - `create_primary_keys()`
  - `create_foreign_keys()`
- ✅ Added `.await` to most database calls
- ✅ Added `.await` to function calls

---

## 🚧 In Progress / TODO

### 5. Script Executor (src/script/executor.rs)
- [ ] Convert `ScriptExecutor::execute()` to async
- [ ] Convert `execute_sql()` to async
- [ ] Convert all meta-command handlers to async
- [ ] Update `ClientState` to work with async executor

### 6. Worker Threads (src/worker/)
- [ ] Convert `thread::run()` to async function
- [ ] Wrap in `tokio::spawn()` for each thread
- [ ] Convert client state machine to async
- [ ] Make `ConnectionState::ExecuteCommand` await queries
- [ ] Enable concurrent client execution within thread using `tokio::select!` or similar
- [ ] This is KEY to fixing the deadlock - multiple clients can overlap I/O

### 7. Main Entry Point (src/main.rs)
- [ ] Add `#[tokio::main]` attribute to `main()` function
- [ ] Convert `main()` to async
- [ ] Add `.await` to `PgBenchConnection::connect()` calls
- [ ] Add `.await` to `initialize_database()` call
- [ ] Add `.await` to benchmark execution

### 8. Statistics & Reporting (src/stats/)
- [ ] Review if any stats collection needs async changes
- [ ] Likely minimal changes needed

### 9. Testing & Validation
- [ ] Fix remaining compilation errors
- [ ] Run `cargo test` and fix any test failures
- [ ] Test with `-c 2 -t 100` (multi-client single-thread)
- [ ] Test with `-c 10 -j 1 -t 100` (many clients, one thread)
- [ ] Verify no deadlocks
- [ ] Performance benchmarking vs synchronous version

### 10. Cleanup
- [ ] Remove temporary deadlock warning from src/cli.rs
- [ ] Update PLAN.md to mark Phase 9.2 as complete
- [ ] Remove src/db/init.rs.backup file
- [ ] Update documentation

---

## Known Compilation Errors

As of last build (14 errors remaining):

1. **Missing `.await` calls**: Some database operations still need `.await`
2. **main.rs not async**: Main function needs `#[tokio::main]`
3. **worker threads sync**: Thread pool still uses blocking operations
4. **script executor sync**: Command execution still synchronous
5. **Unresolved postgres module**: Some files still reference old `postgres` crate

---

## Key Architecture Changes

### Before (Synchronous)
```rust
// Single client blocks entire thread
fn run(client: &mut Client) {
    client.execute("BEGIN").unwrap();  // BLOCKS
    client.execute("UPDATE ...").unwrap();  // BLOCKS waiting for lock
    client.execute("COMMIT").unwrap();  // Never reached if another client holds lock
}
```

### After (Async)
```rust
// Multiple clients can overlap I/O
async fn run(client: &mut Client) {
    client.execute("BEGIN").await.unwrap();  // Yields to other clients
    client.execute("UPDATE ...").await.unwrap();  // Yields while waiting for lock
    client.execute("COMMIT").await.unwrap();  // Can run while other client waits
}

// Thread can manage multiple clients concurrently
tokio::spawn(async move {
    tokio::select! {
        _ = run_client_0() => {},
        _ = run_client_1() => {},
        // Both can make progress!
    }
});
```

---

## Estimated Remaining Work

- **Time**: 4-6 hours of focused work
- **Complexity**: Medium-High (touching many modules)
- **Risk**: Medium (large refactor, but type system catches most errors)

---

## Next Immediate Steps

1. Convert `src/script/executor.rs` to async
2. Convert `src/worker/thread.rs` to use tokio runtime
3. Convert `src/main.rs` to `#[tokio::main]`
4. Fix all compilation errors
5. Test and verify no deadlocks

---

Last Updated: 2025-11-08
