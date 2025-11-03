//! Expression parser
//!
//! Parses pgbench expressions into an AST.

use crate::error::{PgBenchError, PgBenchResult};
use crate::types::PgBenchExpr;

/// Parse an expression string into an AST
pub fn parse_expression(_input: &str) -> PgBenchResult<PgBenchExpr> {
    // TODO: Implement expression parser
    Err(PgBenchError::ExpressionParseError {
        message: "Expression parser not yet implemented".to_string(),
        line: 1,
        column: 1,
    })
}

// TODO: Implement using pest/nom/lalrpop
// TODO: Handle all operators and precedence
// TODO: Support variables
// TODO: Support function calls
// TODO: Provide good error messages
