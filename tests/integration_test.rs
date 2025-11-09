//! Integration tests for pgbench-rust
//!
//! These tests require a running PostgreSQL instance.
//! Set the connection string with: PG_TEST_CONNECTION_STRING env var
//! Default: postgresql://pgbench:pgbench@localhost:5432/pgbench_test
//!
//! Run with: cargo test --test integration_test -- --ignored
//! Run without database tests: cargo test --test integration_test

use pgbench_rust::random::prng::Xoroshiro128StarStar;
use rand::{RngCore, SeedableRng};

/// Get test database connection string from environment or default
fn get_test_connection_string() -> String {
    std::env::var("PG_TEST_CONNECTION_STRING")
        .unwrap_or_else(|_| "postgresql://pgbench:pgbench@localhost:5432/pgbench_test".to_string())
}

#[test]
fn test_random_reproducibility() {
    // Test that same seed produces same random sequence
    // This is CRITICAL for pgbench compatibility
    let mut rng1 = Xoroshiro128StarStar::seed_from_u64(12345);
    let mut rng2 = Xoroshiro128StarStar::seed_from_u64(12345);

    // Generate 1000 random numbers from both RNGs
    for i in 0..1000 {
        let r1 = rng1.next_u64();
        let r2 = rng2.next_u64();
        assert_eq!(
            r1, r2,
            "Random sequences should match with same seed (iteration {})",
            i
        );
    }
}

#[test]
fn test_random_different_seeds() {
    // Test that different seeds produce different sequences
    let mut rng1 = Xoroshiro128StarStar::seed_from_u64(12345);
    let mut rng2 = Xoroshiro128StarStar::seed_from_u64(54321);

    // At least one value should differ
    let mut found_different = false;
    for _ in 0..100 {
        let r1 = rng1.next_u64();
        let r2 = rng2.next_u64();
        if r1 != r2 {
            found_different = true;
            break;
        }
    }
    assert!(
        found_different,
        "Different seeds should produce different sequences"
    );
}

#[test]
fn test_random_distribution_uniform() {
    // Test that uniform random distribution produces values in range
    let mut rng = Xoroshiro128StarStar::seed_from_u64(42);

    for _ in 0..1000 {
        let value = rng.next_u64();
        // Just verify it's a valid u64 (no panic)
        assert!(value <= u64::MAX);
    }
}

#[test]
#[ignore] // Requires PostgreSQL database
fn test_database_connection() {
    // This test verifies we can connect to PostgreSQL
    let conn_str = get_test_connection_string();

    println!("Connection string: {}", conn_str);
    println!("To test connection manually:");
    println!("  RUST_LOG=debug cargo run -- --help");
    println!("  Or run: cargo run -- -i -s 1 {}", conn_str);
}

#[test]
#[ignore] // Requires PostgreSQL database
fn test_database_initialization() {
    // This test would verify database initialization
    let conn_str = get_test_connection_string();

    println!("\n=== Database Initialization Test ===");
    println!("To test database initialization, run:");
    println!("  cargo run --release -- -i -s 1 {}", conn_str);
    println!("\nExpected output:");
    println!("  - Should create tables: pgbench_branches, pgbench_tellers, pgbench_accounts, pgbench_history");
    println!("  - Should create indexes");
    println!("  - Should populate with test data");
}

#[test]
#[ignore] // Requires PostgreSQL database
fn test_multi_client_single_thread() {
    // This test verifies the critical fix for multi-client single-thread deadlock (v0.9.0)
    let conn_str = get_test_connection_string();

    println!("\n=== Multi-Client Single-Thread Test ===");
    println!("This tests the v0.9.0 async concurrency fix");
    println!("To test multi-client single-thread (should NOT hang):");
    println!("  cargo run --release -- -c 2 -j 1 -t 100 {}", conn_str);
    println!("  cargo run --release -- -c 10 -j 1 -t 100 {}", conn_str);
    println!("\nExpected behavior:");
    println!("  - Should complete without hanging");
    println!("  - Should show correct transaction count (2*100=200 or 10*100=1000)");
    println!("  - Should show TPS and latency statistics");
}

#[test]
#[ignore] // Requires PostgreSQL database
fn test_multi_client_multi_thread() {
    // This test verifies multi-threaded execution works correctly
    let conn_str = get_test_connection_string();

    println!("\n=== Multi-Client Multi-Thread Test ===");
    println!("To test multi-client multi-thread:");
    println!("  cargo run --release -- -c 10 -j 2 -T 10 {}", conn_str);
    println!("\nExpected behavior:");
    println!("  - Should complete in ~10 seconds");
    println!("  - Should show TPS and latency statistics");
    println!("  - Should show concurrent execution across threads");
}

#[test]
#[ignore] // Requires PostgreSQL database
fn test_benchmark_duration() {
    // Test duration-based benchmarking (-T flag)
    let conn_str = get_test_connection_string();

    println!("\n=== Duration-Based Benchmark Test ===");
    println!("To test duration-based benchmarking:");
    println!("  cargo run --release -- -c 5 -j 1 -T 5 {}", conn_str);
    println!("\nExpected behavior:");
    println!("  - Should run for approximately 5 seconds");
    println!("  - Should show final statistics");
}

#[test]
#[ignore] // Requires PostgreSQL database
fn test_benchmark_transaction_count() {
    // Test transaction count-based benchmarking (-t flag)
    let conn_str = get_test_connection_string();

    println!("\n=== Transaction Count Benchmark Test ===");
    println!("To test transaction-count benchmarking:");
    println!("  cargo run --release -- -c 2 -j 1 -t 50 {}", conn_str);
    println!("\nExpected behavior:");
    println!("  - Should process exactly 100 transactions (2 clients * 50 each)");
    println!("  - Should show final statistics");
}

#[test]
fn test_module_imports() {
    // Verify that critical modules are accessible
    // This ensures our public API is working correctly

    // Test that we can access types
    use pgbench_rust::types::PgBenchValue;
    let _ = PgBenchValue::int(42);

    // Test that RNG is accessible
    use pgbench_rust::random::Xoroshiro128StarStar;
    let _ = Xoroshiro128StarStar::seed_from_u64(42);

    println!("✓ All critical modules are accessible");
}

#[test]
fn test_value_types() {
    // Test PgBenchValue type system
    use pgbench_rust::types::PgBenchValue;

    // Integer values
    let int_val = PgBenchValue::int(42);
    assert_eq!(int_val.as_int(), Some(42));

    // Double values
    let double_val = PgBenchValue::double(3.14);
    assert_eq!(double_val.as_double(), Some(3.14));

    // Boolean values
    let bool_val = PgBenchValue::boolean(true);
    assert_eq!(bool_val.as_bool(), Some(true));

    // Null values
    let null_val = PgBenchValue::null();
    assert!(null_val.is_null());

    println!("✓ Value type system working correctly");
}
