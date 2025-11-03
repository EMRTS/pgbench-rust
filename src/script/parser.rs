//! Script parser
//!
//! Parses pgbench transaction scripts.

use crate::error::PgBenchResult;
use crate::types::Command;

/// Parse a script file into a list of commands
pub fn parse_script(_input: &str) -> PgBenchResult<Vec<Command>> {
    todo!("Script parser not yet implemented");
}

// TODO: Implement SQL statement parsing
// TODO: Implement meta-command parsing
// TODO: Handle line continuations
// TODO: Handle comments
