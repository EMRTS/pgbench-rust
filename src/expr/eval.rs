//! Expression evaluator
//!
//! Evaluates parsed expressions to produce values.
//!
//! Reference: original-source/pgbench.c lines 2100-2858

use crate::error::{PgBenchError, PgBenchResult};
use crate::types::{PgBenchExpr, PgBenchValue, PgBenchFunction};
use std::collections::HashMap;

/// Context for expression evaluation
pub struct EvalContext {
    /// Variable bindings
    variables: HashMap<String, PgBenchValue>,
}

impl EvalContext {
    /// Create a new evaluation context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Set a variable
    pub fn set_variable(&mut self, name: String, value: PgBenchValue) {
        self.variables.insert(name, value);
    }

    /// Get a variable
    pub fn get_variable(&self, name: &str) -> PgBenchResult<&PgBenchValue> {
        self.variables
            .get(name)
            .ok_or_else(|| PgBenchError::VariableNotFound(name.to_string()))
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate an expression in the given context
///
/// This is the main entry point for expression evaluation.
/// It recursively evaluates the expression tree and returns the result.
///
/// Reference: pgbench.c evaluateExpr() line 2859
pub fn evaluate_expression(
    expr: &PgBenchExpr,
    context: &mut EvalContext,
) -> PgBenchResult<PgBenchValue> {
    match expr {
        // Constants: return the value directly
        PgBenchExpr::Constant(val) => Ok(val.clone()),

        // Variables: lookup in context
        PgBenchExpr::Variable { name } => {
            context.get_variable(name).map(|v| v.clone())
        }

        // Functions and operators
        PgBenchExpr::Function { function, args } => {
            evaluate_function(*function, args, context)
        }
    }
}

/// Evaluate a function or operator
///
/// Reference: pgbench.c evalFunc() line 2843, evalStandardFunc() line 2276, evalLazyFunc() line 2159
fn evaluate_function(
    func: PgBenchFunction,
    args: &[PgBenchExpr],
    context: &mut EvalContext,
) -> PgBenchResult<PgBenchValue> {
    // Special handling for lazy evaluation (AND, OR, CASE)
    if is_lazy_function(func) {
        return evaluate_lazy_function(func, args, context);
    }

    // Evaluate all arguments eagerly for non-lazy functions
    let mut arg_values: Vec<PgBenchValue> = Vec::with_capacity(args.len());
    let mut has_null = false;

    for arg in args {
        let val = evaluate_expression(arg, context)?;
        has_null |= val.is_null();
        arg_values.push(val);
    }

    // Most functions return NULL if any argument is NULL
    // (Exceptions: debug, least, greatest handle NULLs specially)
    if has_null && !func_accepts_null(func) {
        return Ok(PgBenchValue::null());
    }

    // Dispatch to specific function implementation
    evaluate_standard_function(func, &arg_values)
}

/// Check if a function uses lazy evaluation
fn is_lazy_function(func: PgBenchFunction) -> bool {
    matches!(func, PgBenchFunction::And | PgBenchFunction::Or | PgBenchFunction::Case)
}

/// Check if a function accepts NULL arguments
fn func_accepts_null(func: PgBenchFunction) -> bool {
    matches!(func, PgBenchFunction::Debug | PgBenchFunction::Least | PgBenchFunction::Greatest | PgBenchFunction::Is)
}

/// Evaluate lazy functions (AND, OR, CASE)
///
/// These functions don't evaluate all arguments upfront.
/// Reference: pgbench.c evalLazyFunc() line 2159
fn evaluate_lazy_function(
    func: PgBenchFunction,
    args: &[PgBenchExpr],
    context: &mut EvalContext,
) -> PgBenchResult<PgBenchValue> {
    match func {
        PgBenchFunction::And => {
            // AND: short-circuit if first arg is false
            // NULL AND x = NULL, FALSE AND x = FALSE, TRUE AND x = x
            if args.len() != 2 {
                return Err(PgBenchError::invalid_function_args("AND", 2, args.len()));
            }

            let a1 = evaluate_expression(&args[0], context)?;
            if a1.is_null() {
                return Ok(PgBenchValue::null());
            }

            let b1 = a1.coerce_to_bool()?;
            if !b1 {
                return Ok(PgBenchValue::boolean(false));
            }

            let a2 = evaluate_expression(&args[1], context)?;
            if a2.is_null() {
                return Ok(PgBenchValue::null());
            }

            let b2 = a2.coerce_to_bool()?;
            Ok(PgBenchValue::boolean(b2))
        }

        PgBenchFunction::Or => {
            // OR: short-circuit if first arg is true
            // NULL OR x = NULL, TRUE OR x = TRUE, FALSE OR x = x
            if args.len() != 2 {
                return Err(PgBenchError::invalid_function_args("OR", 2, args.len()));
            }

            let a1 = evaluate_expression(&args[0], context)?;
            if a1.is_null() {
                return Ok(PgBenchValue::null());
            }

            let b1 = a1.coerce_to_bool()?;
            if b1 {
                return Ok(PgBenchValue::boolean(true));
            }

            let a2 = evaluate_expression(&args[1], context)?;
            if a2.is_null() {
                return Ok(PgBenchValue::null());
            }

            let b2 = a2.coerce_to_bool()?;
            Ok(PgBenchValue::boolean(b2))
        }

        PgBenchFunction::Case => {
            // CASE: evaluate conditions until one is true
            // Args: [cond1, result1, cond2, result2, ..., else_result]
            // Must have odd number of args (at least 3)
            if args.len() < 3 || args.len() % 2 == 0 {
                return Err(PgBenchError::ExpressionEvalError(
                    "CASE expression requires odd number of arguments (condition, result pairs + else)".to_string()
                ));
            }

            // Iterate through condition/result pairs
            for i in (0..args.len() - 1).step_by(2) {
                let cond = evaluate_expression(&args[i], context)?;

                // Check if condition is true (not NULL and coerces to true)
                if !cond.is_null() && cond.coerce_to_bool()? {
                    return evaluate_expression(&args[i + 1], context);
                }
            }

            // No condition was true, return else result (last arg)
            evaluate_expression(&args[args.len() - 1], context)
        }

        _ => Err(PgBenchError::ExpressionEvalError(
            format!("Internal error: {:?} is not a lazy function", func)
        )),
    }
}

/// Evaluate standard functions (non-lazy)
///
/// All arguments are already evaluated at this point.
/// Reference: pgbench.c evalStandardFunc() line 2276
fn evaluate_standard_function(
    func: PgBenchFunction,
    args: &[PgBenchValue],
) -> PgBenchResult<PgBenchValue> {
    match func {
        // ===== Arithmetic Operators =====
        PgBenchFunction::Add => {
            check_arg_count("addition", 2, args.len())?;
            arithmetic_op(&args[0], &args[1], |a, b| a.checked_add(b), |a, b| a + b)
        }

        PgBenchFunction::Sub => {
            check_arg_count("subtraction", 2, args.len())?;
            arithmetic_op(&args[0], &args[1], |a, b| a.checked_sub(b), |a, b| a - b)
        }

        PgBenchFunction::Mul => {
            check_arg_count("multiplication", 2, args.len())?;
            arithmetic_op(&args[0], &args[1], |a, b| a.checked_mul(b), |a, b| a * b)
        }

        PgBenchFunction::Div => {
            check_arg_count("division", 2, args.len())?;
            division_op(&args[0], &args[1])
        }

        PgBenchFunction::Mod => {
            check_arg_count("modulo", 2, args.len())?;
            modulo_op(&args[0], &args[1])
        }

        // ===== Comparison Operators =====
        PgBenchFunction::Eq => {
            check_arg_count("equality", 2, args.len())?;
            comparison_op(&args[0], &args[1], |cmp| cmp == std::cmp::Ordering::Equal)
        }

        PgBenchFunction::Ne => {
            check_arg_count("inequality", 2, args.len())?;
            comparison_op(&args[0], &args[1], |cmp| cmp != std::cmp::Ordering::Equal)
        }

        PgBenchFunction::Lt => {
            check_arg_count("less than", 2, args.len())?;
            comparison_op(&args[0], &args[1], |cmp| cmp == std::cmp::Ordering::Less)
        }

        PgBenchFunction::Le => {
            check_arg_count("less or equal", 2, args.len())?;
            comparison_op(&args[0], &args[1], |cmp| cmp != std::cmp::Ordering::Greater)
        }

        // ===== Logical Operators =====
        // (AND, OR already handled as lazy functions)

        PgBenchFunction::Not => {
            check_arg_count("NOT", 1, args.len())?;
            let b = args[0].coerce_to_bool()?;
            Ok(PgBenchValue::boolean(!b))
        }

        // ===== Bitwise Operators =====
        PgBenchFunction::BitAnd => {
            check_arg_count("bitwise AND", 2, args.len())?;
            bitwise_op(&args[0], &args[1], |a, b| a & b)
        }

        PgBenchFunction::BitOr => {
            check_arg_count("bitwise OR", 2, args.len())?;
            bitwise_op(&args[0], &args[1], |a, b| a | b)
        }

        PgBenchFunction::BitXor => {
            check_arg_count("bitwise XOR", 2, args.len())?;
            bitwise_op(&args[0], &args[1], |a, b| a ^ b)
        }

        PgBenchFunction::LShift => {
            check_arg_count("left shift", 2, args.len())?;
            shift_op(&args[0], &args[1], true)
        }

        PgBenchFunction::RShift => {
            check_arg_count("right shift", 2, args.len())?;
            shift_op(&args[0], &args[1], false)
        }

        // ===== IS Operator =====
        PgBenchFunction::Is => {
            check_arg_count("IS", 2, args.len())?;
            is_op(&args[0], &args[1])
        }

        // ===== Math Functions =====
        PgBenchFunction::Abs => {
            check_arg_count("abs", 1, args.len())?;
            abs_func(&args[0])
        }

        PgBenchFunction::Sqrt => {
            check_arg_count("sqrt", 1, args.len())?;
            let val = args[0].coerce_to_double()?;
            if val < 0.0 {
                return Err(PgBenchError::invalid_operation("sqrt of negative number"));
            }
            Ok(PgBenchValue::double(val.sqrt()))
        }

        PgBenchFunction::Ln => {
            check_arg_count("ln", 1, args.len())?;
            let val = args[0].coerce_to_double()?;
            if val <= 0.0 {
                return Err(PgBenchError::invalid_operation("ln of non-positive number"));
            }
            Ok(PgBenchValue::double(val.ln()))
        }

        PgBenchFunction::Exp => {
            check_arg_count("exp", 1, args.len())?;
            let val = args[0].coerce_to_double()?;
            Ok(PgBenchValue::double(val.exp()))
        }

        PgBenchFunction::Pow => {
            check_arg_count("pow", 2, args.len())?;
            let base = args[0].coerce_to_double()?;
            let exp = args[1].coerce_to_double()?;
            Ok(PgBenchValue::double(base.powf(exp)))
        }

        PgBenchFunction::Pi => {
            check_arg_count("pi", 0, args.len())?;
            Ok(PgBenchValue::double(std::f64::consts::PI))
        }

        PgBenchFunction::Int => {
            check_arg_count("int", 1, args.len())?;
            let val = args[0].coerce_to_int()?;
            Ok(PgBenchValue::int(val))
        }

        PgBenchFunction::Double => {
            check_arg_count("double", 1, args.len())?;
            let val = args[0].coerce_to_double()?;
            Ok(PgBenchValue::double(val))
        }

        PgBenchFunction::Least => {
            if args.is_empty() {
                return Err(PgBenchError::invalid_function_args("least", 1, 0));
            }
            least_greatest_func(args, true)
        }

        PgBenchFunction::Greatest => {
            if args.is_empty() {
                return Err(PgBenchError::invalid_function_args("greatest", 1, 0));
            }
            least_greatest_func(args, false)
        }

        // ===== Debug Function =====
        PgBenchFunction::Debug => {
            check_arg_count("debug", 1, args.len())?;
            // Print the value and return it
            eprintln!("debug: {}", args[0]);
            Ok(args[0].clone())
        }

        // ===== Not Yet Implemented Functions =====
        PgBenchFunction::Random |
        PgBenchFunction::RandomGaussian |
        PgBenchFunction::RandomExponential |
        PgBenchFunction::RandomZipfian => {
            Err(PgBenchError::ExpressionEvalError(
                format!("Random function {:?} not yet implemented (requires PRNG)", func)
            ))
        }

        PgBenchFunction::HashMurmur2 |
        PgBenchFunction::HashFnv1a => {
            Err(PgBenchError::ExpressionEvalError(
                format!("Hash function {:?} not yet implemented", func)
            ))
        }

        PgBenchFunction::Permute => {
            Err(PgBenchError::ExpressionEvalError(
                "Permute function not yet implemented".to_string()
            ))
        }

        // Lazy functions should not reach here
        PgBenchFunction::And |
        PgBenchFunction::Or |
        PgBenchFunction::Case => {
            Err(PgBenchError::ExpressionEvalError(
                format!("Internal error: lazy function {:?} reached standard eval", func)
            ))
        }
    }
}

// ===== Helper Functions =====

fn check_arg_count(func_name: &str, expected: usize, actual: usize) -> PgBenchResult<()> {
    if actual != expected {
        Err(PgBenchError::invalid_function_args(func_name, expected, actual))
    } else {
        Ok(())
    }
}

/// Arithmetic operation with int/double handling
fn arithmetic_op<F1, F2>(
    a: &PgBenchValue,
    b: &PgBenchValue,
    int_op: F1,
    double_op: F2,
) -> PgBenchResult<PgBenchValue>
where
    F1: FnOnce(i64, i64) -> Option<i64>,
    F2: FnOnce(f64, f64) -> f64,
{
    // Try integer operation first if both are integers
    if let (Some(ia), Some(ib)) = (a.as_int(), b.as_int()) {
        if let Some(result) = int_op(ia, ib) {
            return Ok(PgBenchValue::int(result));
        } else {
            return Err(PgBenchError::operation_overflow("integer arithmetic"));
        }
    }

    // Otherwise, use double
    let da = a.coerce_to_double()?;
    let db = b.coerce_to_double()?;
    Ok(PgBenchValue::double(double_op(da, db)))
}

/// Division operation with special zero handling
fn division_op(a: &PgBenchValue, b: &PgBenchValue) -> PgBenchResult<PgBenchValue> {
    // Try integer division first if both are integers
    if let (Some(ia), Some(ib)) = (a.as_int(), b.as_int()) {
        if ib == 0 {
            return Err(PgBenchError::division_by_zero());
        }
        // Check for overflow: INT64_MIN / -1
        if ia == i64::MIN && ib == -1 {
            return Err(PgBenchError::operation_overflow("integer division"));
        }
        return Ok(PgBenchValue::int(ia / ib));
    }

    // Otherwise, use double
    let da = a.coerce_to_double()?;
    let db = b.coerce_to_double()?;
    if db == 0.0 {
        return Err(PgBenchError::division_by_zero());
    }
    Ok(PgBenchValue::double(da / db))
}

/// Modulo operation
fn modulo_op(a: &PgBenchValue, b: &PgBenchValue) -> PgBenchResult<PgBenchValue> {
    let ia = a.coerce_to_int()?;
    let ib = b.coerce_to_int()?;

    if ib == 0 {
        return Err(PgBenchError::division_by_zero());
    }

    // Rust's % operator can panic on overflow (INT64_MIN % -1)
    if ia == i64::MIN && ib == -1 {
        return Ok(PgBenchValue::int(0));
    }

    Ok(PgBenchValue::int(ia % ib))
}

/// Comparison operation
fn comparison_op<F>(
    a: &PgBenchValue,
    b: &PgBenchValue,
    pred: F,
) -> PgBenchResult<PgBenchValue>
where
    F: FnOnce(std::cmp::Ordering) -> bool,
{
    // Try integer comparison if both are integers
    if let (Some(ia), Some(ib)) = (a.as_int(), b.as_int()) {
        return Ok(PgBenchValue::boolean(pred(ia.cmp(&ib))));
    }

    // Try boolean comparison if both are booleans
    if let (Some(ba), Some(bb)) = (a.as_bool(), b.as_bool()) {
        return Ok(PgBenchValue::boolean(pred(ba.cmp(&bb))));
    }

    // Otherwise, use double comparison
    let da = a.coerce_to_double()?;
    let db = b.coerce_to_double()?;

    // Use partial_cmp for floating point (handles NaN)
    match da.partial_cmp(&db) {
        Some(ordering) => Ok(PgBenchValue::boolean(pred(ordering))),
        None => Ok(PgBenchValue::boolean(false)), // NaN comparisons are always false
    }
}

/// Bitwise operation
fn bitwise_op<F>(
    a: &PgBenchValue,
    b: &PgBenchValue,
    op: F,
) -> PgBenchResult<PgBenchValue>
where
    F: FnOnce(i64, i64) -> i64,
{
    let ia = a.coerce_to_int()?;
    let ib = b.coerce_to_int()?;
    Ok(PgBenchValue::int(op(ia, ib)))
}

/// Shift operation (left or right)
fn shift_op(a: &PgBenchValue, b: &PgBenchValue, left: bool) -> PgBenchResult<PgBenchValue> {
    let value = a.coerce_to_int()?;
    let shift = b.coerce_to_int()?;

    // Validate shift amount
    if shift < 0 {
        return Err(PgBenchError::invalid_operation("negative shift amount"));
    }
    if shift >= 64 {
        // Shifting by >= 64 bits is undefined, return 0
        return Ok(PgBenchValue::int(0));
    }

    let result = if left {
        value.wrapping_shl(shift as u32)
    } else {
        value.wrapping_shr(shift as u32)
    };

    Ok(PgBenchValue::int(result))
}

/// IS operator (identity comparison, including NULL)
fn is_op(a: &PgBenchValue, b: &PgBenchValue) -> PgBenchResult<PgBenchValue> {
    // IS treats NULL specially: NULL IS NULL = true
    if a.is_null() && b.is_null() {
        return Ok(PgBenchValue::boolean(true));
    }
    if a.is_null() || b.is_null() {
        return Ok(PgBenchValue::boolean(false));
    }

    // For non-NULL values, IS is same as =
    // Try exact type match first
    if let (Some(ia), Some(ib)) = (a.as_int(), b.as_int()) {
        return Ok(PgBenchValue::boolean(ia == ib));
    }
    if let (Some(da), Some(db)) = (a.as_double(), b.as_double()) {
        return Ok(PgBenchValue::boolean(da == db));
    }
    if let (Some(ba), Some(bb)) = (a.as_bool(), b.as_bool()) {
        return Ok(PgBenchValue::boolean(ba == bb));
    }

    // Different types, not equal
    Ok(PgBenchValue::boolean(false))
}

/// Absolute value function
fn abs_func(a: &PgBenchValue) -> PgBenchResult<PgBenchValue> {
    // Try integer first
    if let Some(ia) = a.as_int() {
        // Handle INT64_MIN specially (abs(INT64_MIN) overflows)
        if ia == i64::MIN {
            return Err(PgBenchError::operation_overflow("abs(INT64_MIN)"));
        }
        return Ok(PgBenchValue::int(ia.abs()));
    }

    // Otherwise use double
    let da = a.coerce_to_double()?;
    Ok(PgBenchValue::double(da.abs()))
}

/// Least/Greatest function (variable arguments)
fn least_greatest_func(args: &[PgBenchValue], is_least: bool) -> PgBenchResult<PgBenchValue> {
    // Return NULL if any argument is NULL
    if args.iter().any(|v| v.is_null()) {
        return Ok(PgBenchValue::null());
    }

    // Find min/max value
    let mut result = args[0].clone();

    for arg in &args[1..] {
        // Compare result with arg
        let is_less = if let (Some(ir), Some(ia)) = (result.as_int(), arg.as_int()) {
            ir < ia
        } else {
            // Use double comparison
            let dr = result.coerce_to_double()?;
            let da = arg.coerce_to_double()?;
            dr < da
        };

        // Update result based on whether we want least or greatest
        if (is_least && !is_less) || (!is_least && is_less) {
            result = arg.clone();
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create context with variables
    fn context_with_vars(vars: &[(&str, i64)]) -> EvalContext {
        let mut ctx = EvalContext::new();
        for (name, val) in vars {
            ctx.set_variable(name.to_string(), PgBenchValue::int(*val));
        }
        ctx
    }

    #[test]
    fn test_eval_constant() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Constant(PgBenchValue::int(42));
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 42);
    }

    #[test]
    fn test_eval_variable() {
        let mut ctx = context_with_vars(&[("x", 10)]);
        let expr = PgBenchExpr::Variable { name: "x".to_string() };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 10);
    }

    #[test]
    fn test_eval_variable_not_found() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Variable { name: "x".to_string() };
        let result = evaluate_expression(&expr, &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_add() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Add,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(10)),
                PgBenchExpr::Constant(PgBenchValue::int(20)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 30);
    }

    #[test]
    fn test_eval_sub() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Sub,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(100)),
                PgBenchExpr::Constant(PgBenchValue::int(42)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 58);
    }

    #[test]
    fn test_eval_mul() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Mul,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(6)),
                PgBenchExpr::Constant(PgBenchValue::int(7)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 42);
    }

    #[test]
    fn test_eval_div() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Div,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(100)),
                PgBenchExpr::Constant(PgBenchValue::int(5)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 20);
    }

    #[test]
    fn test_eval_div_by_zero() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Div,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(10)),
                PgBenchExpr::Constant(PgBenchValue::int(0)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_mod() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Mod,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(10)),
                PgBenchExpr::Constant(PgBenchValue::int(3)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 1);
    }

    #[test]
    fn test_eval_comparison_lt() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Lt,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(5)),
                PgBenchExpr::Constant(PgBenchValue::int(10)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_bool().unwrap(), true);
    }

    #[test]
    fn test_eval_comparison_eq() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Eq,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(42)),
                PgBenchExpr::Constant(PgBenchValue::int(42)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_bool().unwrap(), true);
    }

    #[test]
    fn test_eval_and_short_circuit() {
        let mut ctx = EvalContext::new();
        // FALSE AND (undefined variable) should not error
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::And,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::boolean(false)),
                PgBenchExpr::Variable { name: "undefined".to_string() },
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_bool().unwrap(), false);
    }

    #[test]
    fn test_eval_or_short_circuit() {
        let mut ctx = EvalContext::new();
        // TRUE OR (undefined variable) should not error
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Or,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::boolean(true)),
                PgBenchExpr::Variable { name: "undefined".to_string() },
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_bool().unwrap(), true);
    }

    #[test]
    fn test_eval_not() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Not,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::boolean(true)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_bool().unwrap(), false);
    }

    #[test]
    fn test_eval_bitwise_and() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::BitAnd,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(5)),  // 0101
                PgBenchExpr::Constant(PgBenchValue::int(3)),  // 0011
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 1);  // 0001
    }

    #[test]
    fn test_eval_bitwise_or() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::BitOr,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(5)),  // 0101
                PgBenchExpr::Constant(PgBenchValue::int(3)),  // 0011
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 7);  // 0111
    }

    #[test]
    fn test_eval_bitwise_xor() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::BitXor,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(5)),  // 0101
                PgBenchExpr::Constant(PgBenchValue::int(3)),  // 0011
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 6);  // 0110
    }

    #[test]
    fn test_eval_left_shift() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::LShift,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(5)),
                PgBenchExpr::Constant(PgBenchValue::int(2)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 20);
    }

    #[test]
    fn test_eval_right_shift() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::RShift,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(20)),
                PgBenchExpr::Constant(PgBenchValue::int(2)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 5);
    }

    #[test]
    fn test_eval_is_null() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Is,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::null()),
                PgBenchExpr::Constant(PgBenchValue::null()),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_bool().unwrap(), true);
    }

    #[test]
    fn test_eval_abs() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Abs,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(-42)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 42);
    }

    #[test]
    fn test_eval_sqrt() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Sqrt,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::double(16.0)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert!((result.as_double().unwrap() - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_eval_pow() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Pow,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(2)),
                PgBenchExpr::Constant(PgBenchValue::int(10)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert!((result.as_double().unwrap() - 1024.0).abs() < 0.001);
    }

    #[test]
    fn test_eval_least() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Least,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(5)),
                PgBenchExpr::Constant(PgBenchValue::int(2)),
                PgBenchExpr::Constant(PgBenchValue::int(8)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 2);
    }

    #[test]
    fn test_eval_greatest() {
        let mut ctx = EvalContext::new();
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Greatest,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(5)),
                PgBenchExpr::Constant(PgBenchValue::int(2)),
                PgBenchExpr::Constant(PgBenchValue::int(8)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 8);
    }

    #[test]
    fn test_eval_case_first_true() {
        let mut ctx = EvalContext::new();
        // CASE WHEN TRUE THEN 1 WHEN TRUE THEN 2 ELSE 3 END
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Case,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::boolean(true)),
                PgBenchExpr::Constant(PgBenchValue::int(1)),
                PgBenchExpr::Constant(PgBenchValue::boolean(true)),
                PgBenchExpr::Constant(PgBenchValue::int(2)),
                PgBenchExpr::Constant(PgBenchValue::int(3)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 1);
    }

    #[test]
    fn test_eval_case_else() {
        let mut ctx = EvalContext::new();
        // CASE WHEN FALSE THEN 1 ELSE 2 END
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Case,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::boolean(false)),
                PgBenchExpr::Constant(PgBenchValue::int(1)),
                PgBenchExpr::Constant(PgBenchValue::int(2)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert_eq!(result.as_int().unwrap(), 2);
    }

    #[test]
    fn test_eval_null_propagation() {
        let mut ctx = EvalContext::new();
        // NULL + 10 should return NULL
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Add,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::null()),
                PgBenchExpr::Constant(PgBenchValue::int(10)),
            ],
        };
        let result = evaluate_expression(&expr, &mut ctx).unwrap();
        assert!(result.is_null());
    }
}
