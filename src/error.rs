//! Error types for pgbench

use thiserror::Error;

/// Main error type for pgbench operations
#[derive(Error, Debug)]
pub enum PgBenchError {
    /// Database connection error
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    /// Query execution error
    #[error("Query execution error: {0}")]
    QueryError(String),

    /// Expression parsing error
    #[error("Expression parsing error at line {line}, column {column}: {message}")]
    ExpressionParseError {
        message: String,
        line: usize,
        column: usize,
    },

    /// Expression evaluation error
    #[error("Expression evaluation error: {0}")]
    ExpressionEvalError(String),

    /// Script parsing error
    #[error("Script parsing error at line {line}: {message}")]
    ScriptParseError {
        message: String,
        line: usize,
    },

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Thread synchronization error
    #[error("Thread synchronization error: {0}")]
    ThreadError(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Random number generation error
    #[error("Random number generation error: {0}")]
    RandomError(String),

    /// Variable not found
    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    /// Type mismatch
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: String,
        actual: String,
    },

    /// Division by zero
    #[error("Division by zero")]
    DivisionByZero,

    /// Integer overflow
    #[error("Integer overflow in operation")]
    IntegerOverflow,

    /// Generic error with message
    #[error("{0}")]
    Generic(String),
}

/// Result type alias for pgbench operations
pub type PgBenchResult<T> = Result<T, PgBenchError>;

impl From<postgres::Error> for PgBenchError {
    fn from(err: postgres::Error) -> Self {
        PgBenchError::QueryError(err.to_string())
    }
}
