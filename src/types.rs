//! Common type definitions for pgbench
//!
//! This module contains the core data structures used throughout pgbench,
//! ported from the C implementation.

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
}
