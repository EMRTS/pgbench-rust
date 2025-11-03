// Build script for pgbench-rust
// This compiles the lalrpop grammar file into Rust code

fn main() {
    // Process lalrpop grammar file
    lalrpop::process_root().expect("Failed to process lalrpop grammar");

    println!("cargo:rerun-if-changed=src/expr/grammar.lalrpop");
}
