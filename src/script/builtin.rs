//! Built-in transaction scripts
//!
//! pgbench provides three built-in transaction scripts that can be used
//! for benchmarking. These match the original C implementation exactly.

/// Scale factors for pgbench tables (per scale unit)
/// These match the original pgbench.c definitions exactly
pub const ACCOUNTS_PER_SCALE: i64 = 100_000;
pub const BRANCHES_PER_SCALE: i64 = 1;
pub const TELLERS_PER_SCALE: i64 = 10;

/// Built-in script definition
#[derive(Debug, Clone)]
pub struct BuiltinScript {
    /// Short name for the script (e.g., "tpcb-like")
    pub name: &'static str,
    /// Description of the script
    pub description: &'static str,
    /// The actual script content (SQL + meta-commands)
    pub script: &'static str,
}

impl BuiltinScript {
    /// Get a built-in script by name
    ///
    /// # Arguments
    /// * `name` - Name of the built-in script
    ///
    /// # Returns
    /// * `Some(script)` if the script exists
    /// * `None` if no script with that name exists
    pub fn get(name: &str) -> Option<&'static BuiltinScript> {
        BUILTIN_SCRIPTS.iter().find(|s| s.name == name)
    }

    /// Get all available built-in scripts
    pub fn all() -> &'static [BuiltinScript] {
        &BUILTIN_SCRIPTS
    }

    /// Check if a name matches a built-in script
    pub fn is_builtin(name: &str) -> bool {
        BUILTIN_SCRIPTS.iter().any(|s| s.name == name)
    }
}

/// TPC-B-like transaction script (default)
///
/// This is the default pgbench workload. It performs:
/// 1. Random variable initialization (aid, bid, tid, delta)
/// 2. UPDATE on accounts table
/// 3. SELECT from accounts table
/// 4. UPDATE on tellers table
/// 5. UPDATE on branches table
/// 6. INSERT into history table
/// All within a single transaction.
///
/// This simulates a banking transaction where money is transferred
/// and the transaction is logged.
const TPCB_LIKE: BuiltinScript = BuiltinScript {
    name: "tpcb-like",
    description: "<builtin: TPC-B (sort of)>",
    script: concat!(
        "\\set aid random(1, ", stringify!(100000), " * :scale)\n",
        "\\set bid random(1, ", stringify!(1), " * :scale)\n",
        "\\set tid random(1, ", stringify!(10), " * :scale)\n",
        "\\set delta random(-5000, 5000)\n",
        "BEGIN;\n",
        "UPDATE pgbench_accounts SET abalance = abalance + :delta WHERE aid = :aid;\n",
        "SELECT abalance FROM pgbench_accounts WHERE aid = :aid;\n",
        "UPDATE pgbench_tellers SET tbalance = tbalance + :delta WHERE tid = :tid;\n",
        "UPDATE pgbench_branches SET bbalance = bbalance + :delta WHERE bid = :bid;\n",
        "INSERT INTO pgbench_history (tid, bid, aid, delta, mtime) VALUES (:tid, :bid, :aid, :delta, CURRENT_TIMESTAMP);\n",
        "END;\n",
    ),
};

/// Simple update transaction script
///
/// This is a simplified version of the TPC-B script that only:
/// 1. Random variable initialization (aid, bid, tid, delta)
/// 2. UPDATE on accounts table
/// 3. SELECT from accounts table
/// 4. INSERT into history table
/// All within a single transaction.
///
/// This omits the tellers and branches updates, making it faster
/// but still representative of a realistic workload.
const SIMPLE_UPDATE: BuiltinScript = BuiltinScript {
    name: "simple-update",
    description: "<builtin: simple update>",
    script: concat!(
        "\\set aid random(1, ", stringify!(100000), " * :scale)\n",
        "\\set bid random(1, ", stringify!(1), " * :scale)\n",
        "\\set tid random(1, ", stringify!(10), " * :scale)\n",
        "\\set delta random(-5000, 5000)\n",
        "BEGIN;\n",
        "UPDATE pgbench_accounts SET abalance = abalance + :delta WHERE aid = :aid;\n",
        "SELECT abalance FROM pgbench_accounts WHERE aid = :aid;\n",
        "INSERT INTO pgbench_history (tid, bid, aid, delta, mtime) VALUES (:tid, :bid, :aid, :delta, CURRENT_TIMESTAMP);\n",
        "END;\n",
    ),
};

/// Select-only transaction script
///
/// This is a read-only workload that only:
/// 1. Random variable initialization (aid)
/// 2. SELECT from accounts table
///
/// This is useful for testing read-only performance and doesn't
/// require a transaction block.
const SELECT_ONLY: BuiltinScript = BuiltinScript {
    name: "select-only",
    description: "<builtin: select only>",
    script: concat!(
        "\\set aid random(1, ", stringify!(100000), " * :scale)\n",
        "SELECT abalance FROM pgbench_accounts WHERE aid = :aid;\n",
    ),
};

/// Array of all built-in scripts
static BUILTIN_SCRIPTS: [BuiltinScript; 3] = [
    TPCB_LIKE,
    SIMPLE_UPDATE,
    SELECT_ONLY,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tpcb_script() {
        let script = BuiltinScript::get("tpcb-like");
        assert!(script.is_some());
        let script = script.unwrap();
        assert_eq!(script.name, "tpcb-like");
        assert!(script.script.contains("BEGIN;"));
        assert!(script.script.contains("UPDATE pgbench_accounts"));
        assert!(script.script.contains("UPDATE pgbench_tellers"));
        assert!(script.script.contains("UPDATE pgbench_branches"));
        assert!(script.script.contains("INSERT INTO pgbench_history"));
        assert!(script.script.contains("END;"));
    }

    #[test]
    fn test_get_simple_update_script() {
        let script = BuiltinScript::get("simple-update");
        assert!(script.is_some());
        let script = script.unwrap();
        assert_eq!(script.name, "simple-update");
        assert!(script.script.contains("BEGIN;"));
        assert!(script.script.contains("UPDATE pgbench_accounts"));
        assert!(!script.script.contains("UPDATE pgbench_tellers"));
        assert!(!script.script.contains("UPDATE pgbench_branches"));
        assert!(script.script.contains("INSERT INTO pgbench_history"));
        assert!(script.script.contains("END;"));
    }

    #[test]
    fn test_get_select_only_script() {
        let script = BuiltinScript::get("select-only");
        assert!(script.is_some());
        let script = script.unwrap();
        assert_eq!(script.name, "select-only");
        assert!(script.script.contains("SELECT abalance"));
        assert!(!script.script.contains("BEGIN;"));
        assert!(!script.script.contains("UPDATE"));
        assert!(!script.script.contains("INSERT"));
    }

    #[test]
    fn test_get_nonexistent_script() {
        let script = BuiltinScript::get("nonexistent");
        assert!(script.is_none());
    }

    #[test]
    fn test_is_builtin() {
        assert!(BuiltinScript::is_builtin("tpcb-like"));
        assert!(BuiltinScript::is_builtin("simple-update"));
        assert!(BuiltinScript::is_builtin("select-only"));
        assert!(!BuiltinScript::is_builtin("custom-script"));
        assert!(!BuiltinScript::is_builtin(""));
    }

    #[test]
    fn test_all_scripts() {
        let scripts = BuiltinScript::all();
        assert_eq!(scripts.len(), 3);
        assert_eq!(scripts[0].name, "tpcb-like");
        assert_eq!(scripts[1].name, "simple-update");
        assert_eq!(scripts[2].name, "select-only");
    }

    #[test]
    fn test_script_constants() {
        // Verify the constants match the original pgbench values
        assert_eq!(ACCOUNTS_PER_SCALE, 100_000);
        assert_eq!(BRANCHES_PER_SCALE, 1);
        assert_eq!(TELLERS_PER_SCALE, 10);
    }

    #[test]
    fn test_tpcb_script_variables() {
        let script = BuiltinScript::get("tpcb-like").unwrap();
        // Should set 4 variables: aid, bid, tid, delta
        assert_eq!(script.script.matches("\\set aid").count(), 1);
        assert_eq!(script.script.matches("\\set bid").count(), 1);
        assert_eq!(script.script.matches("\\set tid").count(), 1);
        assert_eq!(script.script.matches("\\set delta").count(), 1);
        // Should use all 4 variables
        assert!(script.script.contains(":aid"));
        assert!(script.script.contains(":bid"));
        assert!(script.script.contains(":tid"));
        assert!(script.script.contains(":delta"));
    }

    #[test]
    fn test_simple_update_script_variables() {
        let script = BuiltinScript::get("simple-update").unwrap();
        // Should set 4 variables: aid, bid, tid, delta
        assert_eq!(script.script.matches("\\set aid").count(), 1);
        assert_eq!(script.script.matches("\\set bid").count(), 1);
        assert_eq!(script.script.matches("\\set tid").count(), 1);
        assert_eq!(script.script.matches("\\set delta").count(), 1);
    }

    #[test]
    fn test_select_only_script_variables() {
        let script = BuiltinScript::get("select-only").unwrap();
        // Should only set aid variable
        assert_eq!(script.script.matches("\\set aid").count(), 1);
        assert_eq!(script.script.matches("\\set bid").count(), 0);
        assert_eq!(script.script.matches("\\set tid").count(), 0);
        assert_eq!(script.script.matches("\\set delta").count(), 0);
    }

    #[test]
    fn test_script_descriptions() {
        let tpcb = BuiltinScript::get("tpcb-like").unwrap();
        assert!(tpcb.description.contains("TPC-B"));

        let simple = BuiltinScript::get("simple-update").unwrap();
        assert!(simple.description.contains("simple update"));

        let select = BuiltinScript::get("select-only").unwrap();
        assert!(select.description.contains("select only"));
    }
}
