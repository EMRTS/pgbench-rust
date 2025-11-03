//! Expression parser
//!
//! Parses pgbench expressions into an AST using LALRPOP.
//!
//! The grammar is defined in grammar.lalrpop and compiled at build time.

use crate::error::{PgBenchError, PgBenchResult};
use crate::types::PgBenchExpr;

// Include the generated parser module
#[allow(clippy::all)]
mod grammar {
    include!(concat!(env!("OUT_DIR"), "/expr/grammar.rs"));
}

/// Parse an expression string into an AST
///
/// # Examples
///
/// ```ignore
/// use pgbench_rust::expr::parse_expression;
///
/// let expr = parse_expression("1 + 2 * 3").unwrap();
/// ```
pub fn parse_expression(input: &str) -> PgBenchResult<PgBenchExpr> {
    let parser = grammar::ExprParser::new();

    match parser.parse(input) {
        Ok(boxed_expr) => Ok(*boxed_expr),
        Err(e) => {
            // Convert lalrpop error to our error type
            let message = format!("Parse error: {}", e);
            Err(PgBenchError::ExpressionParseError {
                message,
                line: 1, // TODO: Extract line number from lalrpop error
                column: 1, // TODO: Extract column from lalrpop error
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PgBenchFunction, PgBenchValue};

    #[test]
    fn test_parse_integer() {
        let expr = parse_expression("42").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                assert_eq!(val.as_int().unwrap(), 42);
            }
            _ => panic!("Expected integer constant"),
        }
    }

    #[test]
    fn test_parse_double() {
        let expr = parse_expression("3.14").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                let d = val.as_double().unwrap();
                assert!((d - 3.14).abs() < 0.001);
            }
            _ => panic!("Expected double constant"),
        }
    }

    #[test]
    fn test_parse_boolean() {
        let expr = parse_expression("TRUE").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                assert_eq!(val.as_bool().unwrap(), true);
            }
            _ => panic!("Expected boolean constant"),
        }
    }

    #[test]
    fn test_parse_null() {
        let expr = parse_expression("NULL").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                use crate::types::{PgBenchValueData, PgBenchValueType};
                assert_eq!(val.value_type, PgBenchValueType::Null);
            }
            _ => panic!("Expected NULL constant"),
        }
    }

    #[test]
    fn test_parse_variable() {
        let expr = parse_expression(":scale").unwrap();
        match expr {
            PgBenchExpr::Variable { name } => assert_eq!(name, "scale"),
            _ => panic!("Expected variable"),
        }
    }

    #[test]
    fn test_parse_addition() {
        let expr = parse_expression("1 + 2").unwrap();
        match expr {
            PgBenchExpr::Function { function, args } => {
                assert_eq!(function, PgBenchFunction::Add);
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected function call"),
        }
    }

    #[test]
    fn test_parse_precedence() {
        // 1 + 2 * 3 should be 1 + (2 * 3) = 7, not (1 + 2) * 3 = 9
        let expr = parse_expression("1 + 2 * 3").unwrap();
        match expr {
            PgBenchExpr::Function { function, args } => {
                assert_eq!(function, PgBenchFunction::Add);
                // First arg should be 1
                match &args[0] {
                    PgBenchExpr::Constant(val) => {
                        assert_eq!(val.as_int().unwrap(), 1);
                    }
                    _ => panic!("Expected 1 as first argument"),
                }
                // Second arg should be (2 * 3)
                match &args[1] {
                    PgBenchExpr::Function {
                        function: PgBenchFunction::Mul,
                        ..
                    } => {}
                    _ => panic!("Expected multiplication as second argument"),
                }
            }
            _ => panic!("Expected addition at top level"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = parse_expression("(1 + 2) * 3").unwrap();
        match expr {
            PgBenchExpr::Function { function, args } => {
                assert_eq!(function, PgBenchFunction::Mul);
                // First arg should be (1 + 2)
                match &args[0] {
                    PgBenchExpr::Function {
                        function: PgBenchFunction::Add,
                        ..
                    } => {}
                    _ => panic!("Expected addition as first argument"),
                }
            }
            _ => panic!("Expected multiplication at top level"),
        }
    }

    #[test]
    fn test_parse_unary_minus() {
        let expr = parse_expression("-42").unwrap();
        match expr {
            PgBenchExpr::Function { function, args } => {
                assert_eq!(function, PgBenchFunction::Sub);
                assert_eq!(args.len(), 2);
                // First arg should be 0
                match &args[0] {
                    PgBenchExpr::Constant(val) => {
                        assert_eq!(val.as_int().unwrap(), 0);
                    }
                    _ => panic!("Expected 0 as first argument"),
                }
            }
            _ => panic!("Expected subtraction (unary minus)"),
        }
    }

    #[test]
    fn test_parse_comparison() {
        let expr = parse_expression(":x < 10").unwrap();
        match expr {
            PgBenchExpr::Function { function, .. } => {
                assert_eq!(function, PgBenchFunction::Lt);
            }
            _ => panic!("Expected comparison"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let expr = parse_expression("TRUE AND FALSE").unwrap();
        match expr {
            PgBenchExpr::Function { function, .. } => {
                assert_eq!(function, PgBenchFunction::And);
            }
            _ => panic!("Expected logical AND"),
        }
    }
}
