//! Common type definitions for pgbench
//!
//! This module contains the core data structures used throughout pgbench,
//! ported from the C implementation.

use crate::error::{PgBenchError, PgBenchResult};
use std::fmt;

/// Value types supported in pgbench expressions
#[derive(Debug, Clone, PartialEq)]
pub enum PgBenchValueType {
    /// No value / uninitialized
    NoValue,
    /// NULL value
    Null,
    /// 64-bit signed integer
    Int,
    /// 64-bit floating point
    Double,
    /// Boolean value
    Boolean,
}

/// A value in the pgbench expression system
#[derive(Debug, Clone, PartialEq)]
pub struct PgBenchValue {
    pub value_type: PgBenchValueType,
    pub value: PgBenchValueData,
}

/// Union-like enum for value data
#[derive(Debug, Clone, PartialEq)]
pub enum PgBenchValueData {
    NoValue,
    Null,
    Int(i64),
    Double(f64),
    Boolean(bool),
}

impl PgBenchValue {
    /// Create a NULL value
    pub fn null() -> Self {
        Self {
            value_type: PgBenchValueType::Null,
            value: PgBenchValueData::Null,
        }
    }

    /// Create an integer value
    pub fn int(val: i64) -> Self {
        Self {
            value_type: PgBenchValueType::Int,
            value: PgBenchValueData::Int(val),
        }
    }

    /// Create a double value
    pub fn double(val: f64) -> Self {
        Self {
            value_type: PgBenchValueType::Double,
            value: PgBenchValueData::Double(val),
        }
    }

    /// Create a boolean value
    pub fn boolean(val: bool) -> Self {
        Self {
            value_type: PgBenchValueType::Boolean,
            value: PgBenchValueData::Boolean(val),
        }
    }

    /// Check if value is NULL
    pub fn is_null(&self) -> bool {
        matches!(self.value_type, PgBenchValueType::Null)
    }

    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self.value_type {
            PgBenchValueType::NoValue => "no value",
            PgBenchValueType::Null => "null",
            PgBenchValueType::Int => "integer",
            PgBenchValueType::Double => "double",
            PgBenchValueType::Boolean => "boolean",
        }
    }

    /// Coerce value to integer
    pub fn coerce_to_int(&self) -> PgBenchResult<i64> {
        match &self.value {
            PgBenchValueData::Int(v) => Ok(*v),
            PgBenchValueData::Double(v) => {
                // Check for overflow
                if *v >= (i64::MAX as f64) || *v <= (i64::MIN as f64) {
                    Err(PgBenchError::operation_overflow("double to int"))
                } else {
                    Ok(*v as i64)
                }
            }
            PgBenchValueData::Boolean(v) => Ok(if *v { 1 } else { 0 }),
            _ => Err(PgBenchError::coercion_error(self.type_name(), "int")),
        }
    }

    /// Coerce value to double
    pub fn coerce_to_double(&self) -> PgBenchResult<f64> {
        match &self.value {
            PgBenchValueData::Int(v) => Ok(*v as f64),
            PgBenchValueData::Double(v) => Ok(*v),
            PgBenchValueData::Boolean(v) => Ok(if *v { 1.0 } else { 0.0 }),
            _ => Err(PgBenchError::coercion_error(self.type_name(), "double")),
        }
    }

    /// Coerce value to boolean
    pub fn coerce_to_bool(&self) -> PgBenchResult<bool> {
        match &self.value {
            PgBenchValueData::Boolean(v) => Ok(*v),
            PgBenchValueData::Int(v) => Ok(*v != 0),
            _ => Err(PgBenchError::coercion_error(self.type_name(), "boolean")),
        }
    }

    /// Get as integer if possible
    pub fn as_int(&self) -> Option<i64> {
        match self.value {
            PgBenchValueData::Int(v) => Some(v),
            _ => None,
        }
    }

    /// Get as double if possible
    pub fn as_double(&self) -> Option<f64> {
        match self.value {
            PgBenchValueData::Double(v) => Some(v),
            _ => None,
        }
    }

    /// Get as boolean if possible
    pub fn as_bool(&self) -> Option<bool> {
        match self.value {
            PgBenchValueData::Boolean(v) => Some(v),
            _ => None,
        }
    }
}

impl fmt::Display for PgBenchValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            PgBenchValueData::NoValue => write!(f, "<no value>"),
            PgBenchValueData::Null => write!(f, "NULL"),
            PgBenchValueData::Int(v) => write!(f, "{}", v),
            PgBenchValueData::Double(v) => write!(f, "{}", v),
            PgBenchValueData::Boolean(v) => write!(f, "{}", v),
        }
    }
}

/// Expression node types
#[derive(Debug, Clone, PartialEq)]
pub enum PgBenchExprType {
    /// Constant value
    Constant,
    /// Variable reference
    Variable,
    /// Function call
    Function,
}

/// Built-in functions and operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgBenchFunction {
    // Arithmetic operators
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Math functions
    Abs,
    Least,
    Greatest,
    Int,
    Double,
    Pi,
    Sqrt,
    Ln,
    Exp,
    Pow,

    // Random functions
    Random,
    RandomGaussian,
    RandomExponential,
    RandomZipfian,

    // Logical operators
    And,
    Or,
    Not,

    // Bitwise operators
    BitAnd,
    BitOr,
    BitXor,
    LShift,
    RShift,

    // Comparison operators
    Eq,
    Ne,
    Le,
    Lt,
    Is,

    // Hash functions
    HashFnv1a,
    HashMurmur2,

    // Other
    Debug,
    Case,
    Permute,
}

/// Expression tree node
#[derive(Debug, Clone, PartialEq)]
pub enum PgBenchExpr {
    /// Constant value
    Constant(PgBenchValue),

    /// Variable reference
    Variable {
        name: String,
    },

    /// Function call
    Function {
        function: PgBenchFunction,
        args: Vec<PgBenchExpr>,
    },
}

/// Transaction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMode {
    /// Simple query protocol
    Simple,
    /// Extended query protocol
    Extended,
    /// Prepared statements
    Prepared,
}

/// Meta-command types in scripts
#[derive(Debug, Clone, PartialEq)]
pub enum MetaCommand {
    /// \set variable value
    Set { variable: String, value: String },

    /// \setshell variable command
    SetShell { variable: String, command: String },

    /// \sleep duration
    Sleep { duration: f64 },

    /// \if condition
    If { condition: PgBenchExpr },

    /// \elif condition
    ElseIf { condition: PgBenchExpr },

    /// \else
    Else,

    /// \endif
    EndIf,

    /// \startpipeline
    StartPipeline,

    /// \endpipeline
    EndPipeline,
}

/// A command in a transaction script
#[derive(Debug, Clone)]
pub enum Command {
    /// SQL statement
    Sql {
        query: String,
    },

    /// Meta-command
    Meta(MetaCommand),
}

/// Statistics for a single transaction
#[derive(Debug, Clone, Copy, Default)]
pub struct TransactionStats {
    /// Number of transactions executed
    pub count: u64,

    /// Total time in microseconds
    pub total_time: u64,

    /// Sum of squared times (for stddev calculation)
    pub sum_squared: f64,

    /// Minimum latency
    pub min_latency: u64,

    /// Maximum latency
    pub max_latency: u64,
}

/// Thread state for a worker thread
#[derive(Debug)]
pub struct ThreadState {
    /// Thread ID
    pub id: usize,

    /// Random number generator state
    pub rng_state: u64,

    /// Statistics for this thread
    pub stats: TransactionStats,

    /// Number of transactions to execute (0 = unlimited)
    pub ntransactions: u64,

    /// Start time
    pub start_time: std::time::Instant,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pgbench_value_creation() {
        let null_val = PgBenchValue::null();
        assert!(null_val.is_null());

        let int_val = PgBenchValue::int(42);
        assert!(!int_val.is_null());
        assert_eq!(int_val.value, PgBenchValueData::Int(42));

        let double_val = PgBenchValue::double(3.14);
        assert_eq!(double_val.value, PgBenchValueData::Double(3.14));

        let bool_val = PgBenchValue::boolean(true);
        assert_eq!(bool_val.value, PgBenchValueData::Boolean(true));
    }

    #[test]
    fn test_value_display() {
        assert_eq!(PgBenchValue::null().to_string(), "NULL");
        assert_eq!(PgBenchValue::int(42).to_string(), "42");
        assert_eq!(PgBenchValue::double(3.14).to_string(), "3.14");
        assert_eq!(PgBenchValue::boolean(true).to_string(), "true");
    }

    #[test]
    fn test_type_name() {
        assert_eq!(PgBenchValue::null().type_name(), "null");
        assert_eq!(PgBenchValue::int(42).type_name(), "integer");
        assert_eq!(PgBenchValue::double(3.14).type_name(), "double");
        assert_eq!(PgBenchValue::boolean(true).type_name(), "boolean");
    }

    #[test]
    fn test_coerce_to_int() {
        // Int to int (identity)
        assert_eq!(PgBenchValue::int(42).coerce_to_int().unwrap(), 42);

        // Double to int
        assert_eq!(PgBenchValue::double(3.14).coerce_to_int().unwrap(), 3);
        assert_eq!(PgBenchValue::double(-5.9).coerce_to_int().unwrap(), -5);

        // Boolean to int
        assert_eq!(PgBenchValue::boolean(true).coerce_to_int().unwrap(), 1);
        assert_eq!(PgBenchValue::boolean(false).coerce_to_int().unwrap(), 0);

        // Null to int should fail
        assert!(PgBenchValue::null().coerce_to_int().is_err());

        // Overflow check
        let large_double = (i64::MAX as f64) * 2.0;
        assert!(PgBenchValue::double(large_double).coerce_to_int().is_err());
    }

    #[test]
    fn test_coerce_to_double() {
        // Double to double (identity)
        assert_eq!(PgBenchValue::double(3.14).coerce_to_double().unwrap(), 3.14);

        // Int to double
        assert_eq!(PgBenchValue::int(42).coerce_to_double().unwrap(), 42.0);

        // Boolean to double
        assert_eq!(PgBenchValue::boolean(true).coerce_to_double().unwrap(), 1.0);
        assert_eq!(PgBenchValue::boolean(false).coerce_to_double().unwrap(), 0.0);

        // Null to double should fail
        assert!(PgBenchValue::null().coerce_to_double().is_err());
    }

    #[test]
    fn test_coerce_to_bool() {
        // Bool to bool (identity)
        assert_eq!(PgBenchValue::boolean(true).coerce_to_bool().unwrap(), true);
        assert_eq!(PgBenchValue::boolean(false).coerce_to_bool().unwrap(), false);

        // Int to bool
        assert_eq!(PgBenchValue::int(1).coerce_to_bool().unwrap(), true);
        assert_eq!(PgBenchValue::int(0).coerce_to_bool().unwrap(), false);
        assert_eq!(PgBenchValue::int(42).coerce_to_bool().unwrap(), true);
        assert_eq!(PgBenchValue::int(-1).coerce_to_bool().unwrap(), true);

        // Double to bool should fail
        assert!(PgBenchValue::double(1.0).coerce_to_bool().is_err());

        // Null to bool should fail
        assert!(PgBenchValue::null().coerce_to_bool().is_err());
    }

    #[test]
    fn test_as_accessors() {
        let int_val = PgBenchValue::int(42);
        assert_eq!(int_val.as_int(), Some(42));
        assert_eq!(int_val.as_double(), None);
        assert_eq!(int_val.as_bool(), None);

        let double_val = PgBenchValue::double(3.14);
        assert_eq!(double_val.as_int(), None);
        assert_eq!(double_val.as_double(), Some(3.14));
        assert_eq!(double_val.as_bool(), None);

        let bool_val = PgBenchValue::boolean(true);
        assert_eq!(bool_val.as_int(), None);
        assert_eq!(bool_val.as_double(), None);
        assert_eq!(bool_val.as_bool(), Some(true));
    }

    #[test]
    fn test_expr_constant() {
        let expr = PgBenchExpr::Constant(PgBenchValue::int(42));
        match expr {
            PgBenchExpr::Constant(val) => assert_eq!(val.as_int(), Some(42)),
            _ => panic!("Expected constant expression"),
        }
    }

    #[test]
    fn test_expr_variable() {
        let expr = PgBenchExpr::Variable {
            name: "myvar".to_string(),
        };
        match expr {
            PgBenchExpr::Variable { name } => assert_eq!(name, "myvar"),
            _ => panic!("Expected variable expression"),
        }
    }

    #[test]
    fn test_expr_function() {
        let expr = PgBenchExpr::Function {
            function: PgBenchFunction::Add,
            args: vec![
                PgBenchExpr::Constant(PgBenchValue::int(1)),
                PgBenchExpr::Constant(PgBenchValue::int(2)),
            ],
        };
        match expr {
            PgBenchExpr::Function { function, args } => {
                assert_eq!(function, PgBenchFunction::Add);
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected function expression"),
        }
    }

    #[test]
    fn test_transaction_stats_default() {
        let stats = TransactionStats::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.total_time, 0);
        assert_eq!(stats.sum_squared, 0.0);
        assert_eq!(stats.min_latency, 0);
        assert_eq!(stats.max_latency, 0);
    }

    #[test]
    fn test_transaction_mode() {
        let mode = TransactionMode::Simple;
        assert_eq!(mode, TransactionMode::Simple);

        let mode = TransactionMode::Extended;
        assert_eq!(mode, TransactionMode::Extended);

        let mode = TransactionMode::Prepared;
        assert_eq!(mode, TransactionMode::Prepared);
    }

    #[test]
    fn test_meta_command_set() {
        let cmd = MetaCommand::Set {
            variable: "myvar".to_string(),
            value: "42".to_string(),
        };
        match cmd {
            MetaCommand::Set { variable, value } => {
                assert_eq!(variable, "myvar");
                assert_eq!(value, "42");
            }
            _ => panic!("Expected Set meta-command"),
        }
    }

    #[test]
    fn test_meta_command_sleep() {
        let cmd = MetaCommand::Sleep { duration: 1.5 };
        match cmd {
            MetaCommand::Sleep { duration } => assert_eq!(duration, 1.5),
            _ => panic!("Expected Sleep meta-command"),
        }
    }

    #[test]
    fn test_command_sql() {
        let cmd = Command::Sql {
            query: "SELECT 1".to_string(),
        };
        match cmd {
            Command::Sql { query } => assert_eq!(query, "SELECT 1"),
            _ => panic!("Expected SQL command"),
        }
    }

    #[test]
    fn test_command_meta() {
        let meta = MetaCommand::Else;
        let cmd = Command::Meta(meta.clone());
        match cmd {
            Command::Meta(m) => match m {
                MetaCommand::Else => (),
                _ => panic!("Expected Else meta-command"),
            },
            _ => panic!("Expected Meta command"),
        }
    }
}
