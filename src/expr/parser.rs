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
    use crate::types::{PgBenchFunction, PgBenchValue, PgBenchValueType};

    // Helper to verify function type
    fn assert_function(expr: &PgBenchExpr, expected_fn: PgBenchFunction) {
        match expr {
            PgBenchExpr::Function { function, .. } => {
                assert_eq!(*function, expected_fn);
            }
            _ => panic!("Expected function call, got {:?}", expr),
        }
    }

    // === Basic Constants ===

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
    fn test_parse_double_scientific() {
        let expr = parse_expression("1.5e10").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                let d = val.as_double().unwrap();
                assert!((d - 1.5e10).abs() < 1e6);
            }
            _ => panic!("Expected double constant"),
        }
    }

    #[test]
    fn test_parse_double_negative_exponent() {
        let expr = parse_expression("2.5e-3").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                let d = val.as_double().unwrap();
                assert!((d - 0.0025).abs() < 1e-6);
            }
            _ => panic!("Expected double constant"),
        }
    }

    #[test]
    fn test_parse_boolean_true() {
        let expr = parse_expression("TRUE").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                assert_eq!(val.as_bool().unwrap(), true);
            }
            _ => panic!("Expected boolean constant"),
        }
    }

    #[test]
    fn test_parse_boolean_false() {
        let expr = parse_expression("FALSE").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                assert_eq!(val.as_bool().unwrap(), false);
            }
            _ => panic!("Expected boolean constant"),
        }
    }

    #[test]
    fn test_parse_null() {
        let expr = parse_expression("NULL").unwrap();
        match expr {
            PgBenchExpr::Constant(val) => {
                assert_eq!(val.value_type, PgBenchValueType::Null);
            }
            _ => panic!("Expected NULL constant"),
        }
    }

    // === Variables ===

    #[test]
    fn test_parse_variable() {
        let expr = parse_expression(":scale").unwrap();
        match expr {
            PgBenchExpr::Variable { name } => assert_eq!(name, "scale"),
            _ => panic!("Expected variable"),
        }
    }

    #[test]
    fn test_parse_variable_with_underscore() {
        let expr = parse_expression(":my_var_123").unwrap();
        match expr {
            PgBenchExpr::Variable { name } => assert_eq!(name, "my_var_123"),
            _ => panic!("Expected variable"),
        }
    }

    // === Arithmetic Operators ===

    #[test]
    fn test_parse_addition() {
        let expr = parse_expression("1 + 2").unwrap();
        assert_function(&expr, PgBenchFunction::Add);
    }

    #[test]
    fn test_parse_subtraction() {
        let expr = parse_expression("5 - 3").unwrap();
        assert_function(&expr, PgBenchFunction::Sub);
    }

    #[test]
    fn test_parse_multiplication() {
        let expr = parse_expression("4 * 5").unwrap();
        assert_function(&expr, PgBenchFunction::Mul);
    }

    #[test]
    fn test_parse_division() {
        let expr = parse_expression("10 / 2").unwrap();
        assert_function(&expr, PgBenchFunction::Div);
    }

    #[test]
    fn test_parse_modulo() {
        let expr = parse_expression("10 % 3").unwrap();
        assert_function(&expr, PgBenchFunction::Mod);
    }

    // === Operator Precedence ===

    #[test]
    fn test_parse_precedence_mul_before_add() {
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
    fn test_parse_unary_plus() {
        let expr = parse_expression("+42").unwrap();
        // Unary plus is identity, so should just be 42
        match expr {
            PgBenchExpr::Constant(val) => {
                assert_eq!(val.as_int().unwrap(), 42);
            }
            _ => panic!("Expected integer constant"),
        }
    }

    // === Comparison Operators ===

    #[test]
    fn test_parse_less_than() {
        let expr = parse_expression(":x < 10").unwrap();
        assert_function(&expr, PgBenchFunction::Lt);
    }

    #[test]
    fn test_parse_less_equal() {
        let expr = parse_expression(":x <= 10").unwrap();
        assert_function(&expr, PgBenchFunction::Le);
    }

    #[test]
    fn test_parse_greater_than() {
        // > is implemented as < with swapped arguments
        let expr = parse_expression(":x > 10").unwrap();
        assert_function(&expr, PgBenchFunction::Lt);
    }

    #[test]
    fn test_parse_greater_equal() {
        // >= is implemented as <= with swapped arguments
        let expr = parse_expression(":x >= 10").unwrap();
        assert_function(&expr, PgBenchFunction::Le);
    }

    #[test]
    fn test_parse_equal() {
        let expr = parse_expression(":x = 10").unwrap();
        assert_function(&expr, PgBenchFunction::Eq);
    }

    #[test]
    fn test_parse_not_equal_diamond() {
        let expr = parse_expression(":x <> 10").unwrap();
        assert_function(&expr, PgBenchFunction::Ne);
    }

    #[test]
    fn test_parse_not_equal_exclaim() {
        let expr = parse_expression(":x != 10").unwrap();
        assert_function(&expr, PgBenchFunction::Ne);
    }

    // === Logical Operators ===

    #[test]
    fn test_parse_logical_and() {
        let expr = parse_expression("TRUE AND FALSE").unwrap();
        assert_function(&expr, PgBenchFunction::And);
    }

    #[test]
    fn test_parse_logical_or() {
        let expr = parse_expression("TRUE OR FALSE").unwrap();
        assert_function(&expr, PgBenchFunction::Or);
    }

    #[test]
    fn test_parse_logical_not() {
        let expr = parse_expression("NOT TRUE").unwrap();
        assert_function(&expr, PgBenchFunction::Not);
    }

    // === Bitwise Operators ===

    #[test]
    fn test_parse_bitwise_and() {
        let expr = parse_expression("5 & 3").unwrap();
        assert_function(&expr, PgBenchFunction::BitAnd);
    }

    #[test]
    fn test_parse_bitwise_or() {
        let expr = parse_expression("5 | 3").unwrap();
        assert_function(&expr, PgBenchFunction::BitOr);
    }

    #[test]
    fn test_parse_bitwise_xor() {
        let expr = parse_expression("5 # 3").unwrap();
        assert_function(&expr, PgBenchFunction::BitXor);
    }

    #[test]
    fn test_parse_left_shift() {
        let expr = parse_expression("5 << 2").unwrap();
        assert_function(&expr, PgBenchFunction::LShift);
    }

    #[test]
    fn test_parse_right_shift() {
        let expr = parse_expression("20 >> 2").unwrap();
        assert_function(&expr, PgBenchFunction::RShift);
    }

    #[test]
    fn test_parse_bitwise_complement() {
        let expr = parse_expression("~42").unwrap();
        // ~ is implemented as XOR with all bits set
        assert_function(&expr, PgBenchFunction::BitXor);
    }

    // === IS Operators ===

    #[test]
    fn test_parse_is_null() {
        let expr = parse_expression(":x IS NULL").unwrap();
        assert_function(&expr, PgBenchFunction::Is);
    }

    #[test]
    fn test_parse_is_not_null() {
        let expr = parse_expression(":x IS NOT NULL").unwrap();
        // IS NOT NULL is implemented as NOT (IS NULL)
        assert_function(&expr, PgBenchFunction::Not);
    }

    #[test]
    fn test_parse_is_true() {
        let expr = parse_expression(":x IS TRUE").unwrap();
        assert_function(&expr, PgBenchFunction::Is);
    }

    #[test]
    fn test_parse_is_not_true() {
        let expr = parse_expression(":x IS NOT TRUE").unwrap();
        assert_function(&expr, PgBenchFunction::Not);
    }

    #[test]
    fn test_parse_is_false() {
        let expr = parse_expression(":x IS FALSE").unwrap();
        assert_function(&expr, PgBenchFunction::Is);
    }

    #[test]
    fn test_parse_is_not_false() {
        let expr = parse_expression(":x IS NOT FALSE").unwrap();
        assert_function(&expr, PgBenchFunction::Not);
    }

    // === Built-in Functions ===

    #[test]
    fn test_parse_function_no_args() {
        let expr = parse_expression("pi()").unwrap();
        assert_function(&expr, PgBenchFunction::Pi);
    }

    #[test]
    fn test_parse_function_one_arg() {
        let expr = parse_expression("abs(-5)").unwrap();
        assert_function(&expr, PgBenchFunction::Abs);
    }

    #[test]
    fn test_parse_function_two_args() {
        let expr = parse_expression("pow(2, 10)").unwrap();
        assert_function(&expr, PgBenchFunction::Pow);
    }

    #[test]
    fn test_parse_function_power_alias() {
        let expr = parse_expression("power(2, 10)").unwrap();
        assert_function(&expr, PgBenchFunction::Pow);
    }

    #[test]
    fn test_parse_math_functions() {
        assert_function(&parse_expression("sqrt(16)").unwrap(), PgBenchFunction::Sqrt);
        assert_function(&parse_expression("ln(10)").unwrap(), PgBenchFunction::Ln);
        assert_function(&parse_expression("exp(2)").unwrap(), PgBenchFunction::Exp);
        assert_function(&parse_expression("int(3.14)").unwrap(), PgBenchFunction::Int);
        assert_function(&parse_expression("double(42)").unwrap(), PgBenchFunction::Double);
    }

    #[test]
    fn test_parse_least_greatest() {
        assert_function(&parse_expression("least(1, 2, 3)").unwrap(), PgBenchFunction::Least);
        assert_function(&parse_expression("greatest(1, 2, 3)").unwrap(), PgBenchFunction::Greatest);
    }

    #[test]
    fn test_parse_random_functions() {
        assert_function(&parse_expression("random(1, 100)").unwrap(), PgBenchFunction::Random);
        assert_function(&parse_expression("random_gaussian(0, 100, 2.5)").unwrap(), PgBenchFunction::RandomGaussian);
        assert_function(&parse_expression("random_exponential(0, 100, 2.5)").unwrap(), PgBenchFunction::RandomExponential);
        assert_function(&parse_expression("random_zipfian(0, 100, 2.5)").unwrap(), PgBenchFunction::RandomZipfian);
    }

    #[test]
    fn test_parse_hash_functions() {
        assert_function(&parse_expression("hash(42)").unwrap(), PgBenchFunction::HashMurmur2);
        assert_function(&parse_expression("hash_murmur2(42)").unwrap(), PgBenchFunction::HashMurmur2);
        assert_function(&parse_expression("hash_fnv1a(42)").unwrap(), PgBenchFunction::HashFnv1a);
    }

    #[test]
    fn test_parse_debug() {
        assert_function(&parse_expression("debug(42)").unwrap(), PgBenchFunction::Debug);
    }

    #[test]
    fn test_parse_permute() {
        assert_function(&parse_expression("permute(42, 100)").unwrap(), PgBenchFunction::Permute);
    }

    // === CASE Expressions ===

    #[test]
    fn test_parse_case_simple() {
        let expr = parse_expression("CASE WHEN :x > 0 THEN 1 END").unwrap();
        assert_function(&expr, PgBenchFunction::Case);
        match expr {
            PgBenchExpr::Function { args, .. } => {
                // Should have 3 args: condition, result, NULL
                assert_eq!(args.len(), 3);
                // Last arg should be NULL
                match &args[2] {
                    PgBenchExpr::Constant(val) => {
                        assert_eq!(val.value_type, PgBenchValueType::Null);
                    }
                    _ => panic!("Expected NULL as last arg"),
                }
            }
            _ => panic!("Expected CASE function"),
        }
    }

    #[test]
    fn test_parse_case_with_else() {
        let expr = parse_expression("CASE WHEN :x > 0 THEN 1 ELSE 0 END").unwrap();
        assert_function(&expr, PgBenchFunction::Case);
        match expr {
            PgBenchExpr::Function { args, .. } => {
                // Should have 3 args: condition, result, else_value
                assert_eq!(args.len(), 3);
                // Last arg should be 0
                match &args[2] {
                    PgBenchExpr::Constant(val) => {
                        assert_eq!(val.as_int().unwrap(), 0);
                    }
                    _ => panic!("Expected 0 as else value"),
                }
            }
            _ => panic!("Expected CASE function"),
        }
    }

    #[test]
    fn test_parse_case_multiple_when() {
        let expr = parse_expression("CASE WHEN :x < 0 THEN -1 WHEN :x > 0 THEN 1 ELSE 0 END").unwrap();
        assert_function(&expr, PgBenchFunction::Case);
        match expr {
            PgBenchExpr::Function { args, .. } => {
                // Should have 5 args: cond1, result1, cond2, result2, else_value
                assert_eq!(args.len(), 5);
            }
            _ => panic!("Expected CASE function"),
        }
    }

    // === Complex Expressions ===

    #[test]
    fn test_parse_complex_arithmetic() {
        // (1 + 2) * 3 - 4 / 2
        let expr = parse_expression("(1 + 2) * 3 - 4 / 2").unwrap();
        // Top level should be subtraction
        assert_function(&expr, PgBenchFunction::Sub);
    }

    #[test]
    fn test_parse_complex_with_variables() {
        let expr = parse_expression(":scale * 100000 + :client_id").unwrap();
        assert_function(&expr, PgBenchFunction::Add);
    }

    #[test]
    fn test_parse_nested_functions() {
        let expr = parse_expression("abs(sqrt(pow(:x, 2)))").unwrap();
        assert_function(&expr, PgBenchFunction::Abs);
    }

    #[test]
    fn test_parse_logical_precedence() {
        // :x > 0 AND :y > 0 OR :z > 0
        // Should parse as (:x > 0 AND :y > 0) OR :z > 0
        let expr = parse_expression(":x > 0 AND :y > 0 OR :z > 0").unwrap();
        assert_function(&expr, PgBenchFunction::Or);
    }

    #[test]
    fn test_parse_bitwise_precedence() {
        // 1 + 2 & 3 should be (1 + 2) & 3, not 1 + (2 & 3)
        // Because + has higher precedence than &
        let expr = parse_expression("1 + 2 & 3").unwrap();
        assert_function(&expr, PgBenchFunction::BitAnd);
    }

    // === Whitespace and Comments ===

    #[test]
    fn test_parse_with_whitespace() {
        let expr = parse_expression("  1   +   2  ").unwrap();
        assert_function(&expr, PgBenchFunction::Add);
    }

    #[test]
    fn test_parse_with_sql_comment() {
        let expr = parse_expression("1 + 2 -- this is a comment\n").unwrap();
        assert_function(&expr, PgBenchFunction::Add);
    }

    #[test]
    fn test_parse_with_c_comment() {
        let expr = parse_expression("1 /* comment */ + /* another */ 2").unwrap();
        assert_function(&expr, PgBenchFunction::Add);
    }

    // === Error Cases ===

    #[test]
    fn test_parse_invalid_syntax() {
        let result = parse_expression("1 + + 2");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_function() {
        let result = parse_expression("unknown_func(1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unclosed_paren() {
        let result = parse_expression("(1 + 2");
        assert!(result.is_err());
    }
}
