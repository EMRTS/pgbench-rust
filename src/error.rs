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

    /// Query execution error with context
    #[error("Query failed: {message}\nQuery was: {query}")]
    QueryErrorWithContext { message: String, query: String },

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

    /// Script execution error with context
    #[error("Condition error in script \"{script}\" command {command}: {message}")]
    ScriptExecutionError {
        script: String,
        command: usize,
        message: String,
    },

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// File operation error with context
    #[error("Could not {operation} file \"{path}\": {message}")]
    FileError {
        operation: String,
        path: String,
        message: String,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Thread synchronization error
    #[error("Thread synchronization error: {0}")]
    ThreadError(String),

    /// Thread panic
    #[error("Thread panicked: {0}")]
    ThreadPanic(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Invalid input syntax
    #[error("Invalid input syntax for type {type_name}: \"{value}\"")]
    InvalidInputSyntax { type_name: String, value: String },

    /// Value out of range
    #[error("Value \"{value}\" is out of range for type {type_name}")]
    ValueOutOfRange { value: String, type_name: String },

    /// Random number generation error
    #[error("Random number generation error: {0}")]
    RandomError(String),

    /// Invalid random parameter
    #[error("Invalid random parameter: {0}")]
    InvalidRandomParameter(String),

    /// Empty range error
    #[error("Empty range given to random")]
    EmptyRange,

    /// Range too large
    #[error("Random range is too large")]
    RangeTooLarge,

    /// Variable not found
    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    /// Invalid variable name
    #[error("Invalid variable name: \"{0}\"")]
    InvalidVariableName(String),

    /// Malformed variable value
    #[error("Malformed variable \"{name}\" value: \"{value}\"")]
    MalformedVariableValue { name: String, value: String },

    /// Type mismatch
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: String,
        actual: String,
    },

    /// Type coercion error
    #[error("Cannot coerce {from_type} to {to_type}")]
    CoercionError {
        from_type: String,
        to_type: String,
    },

    /// Division by zero
    #[error("Division by zero")]
    DivisionByZero,

    /// Integer overflow
    #[error("Integer overflow in operation")]
    IntegerOverflow,

    /// Integer overflow with operation details
    #[error("{operation} overflow")]
    OperationOverflow { operation: String },

    /// Initialization error
    #[error("Initialization error: {0}")]
    InitializationError(String),

    /// No initialization steps specified
    #[error("No initialization steps specified")]
    NoInitializationSteps,

    /// Partition error
    #[error("Partition error: {0}")]
    PartitionError(String),

    /// Invalid weight specification
    #[error("Invalid weight specification: {0}")]
    InvalidWeight(String),

    /// Empty command list
    #[error("Empty command list for script \"{0}\"")]
    EmptyCommandList(String),

    /// Too many scripts
    #[error("At most {max} SQL scripts are allowed")]
    TooManyScripts { max: usize },

    /// Too many function arguments
    #[error("Too many function arguments, maximum is {max}")]
    TooManyFunctionArgs { max: usize },

    /// Unexpected error status
    #[error("Unexpected error status: {0}")]
    UnexpectedStatus(i32),

    /// Unexpected node type
    #[error("Unexpected node type in evaluation: {0}")]
    UnexpectedNodeType(i32),

    /// Generic error with message
    #[error("{0}")]
    Generic(String),
}

/// Result type alias for pgbench operations
pub type PgBenchResult<T> = Result<T, PgBenchError>;

// From trait implementations for external error types
impl From<postgres::Error> for PgBenchError {
    fn from(err: postgres::Error) -> Self {
        PgBenchError::QueryError(err.to_string())
    }
}

impl From<anyhow::Error> for PgBenchError {
    fn from(err: anyhow::Error) -> Self {
        PgBenchError::Generic(err.to_string())
    }
}

impl From<std::num::ParseIntError> for PgBenchError {
    fn from(err: std::num::ParseIntError) -> Self {
        PgBenchError::InvalidInputSyntax {
            type_name: "integer".to_string(),
            value: err.to_string(),
        }
    }
}

impl From<std::num::ParseFloatError> for PgBenchError {
    fn from(err: std::num::ParseFloatError) -> Self {
        PgBenchError::InvalidInputSyntax {
            type_name: "float".to_string(),
            value: err.to_string(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for PgBenchError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        PgBenchError::ThreadError(format!("Mutex poisoned: {}", err))
    }
}

// Error context helpers
impl PgBenchError {
    /// Create a query error with SQL context
    pub fn query_with_context(message: impl Into<String>, query: impl Into<String>) -> Self {
        Self::QueryErrorWithContext {
            message: message.into(),
            query: query.into(),
        }
    }

    /// Create a file error with operation context
    pub fn file_error(
        operation: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::FileError {
            operation: operation.into(),
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a script execution error with context
    pub fn script_error(
        script: impl Into<String>,
        command: usize,
        message: impl Into<String>,
    ) -> Self {
        Self::ScriptExecutionError {
            script: script.into(),
            command,
            message: message.into(),
        }
    }

    /// Create a malformed variable error
    pub fn malformed_variable(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::MalformedVariableValue {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Create a coercion error
    pub fn coercion_error(from_type: impl Into<String>, to_type: impl Into<String>) -> Self {
        Self::CoercionError {
            from_type: from_type.into(),
            to_type: to_type.into(),
        }
    }

    /// Create an operation overflow error
    pub fn operation_overflow(operation: impl Into<String>) -> Self {
        Self::OperationOverflow {
            operation: operation.into(),
        }
    }

    /// Create a division by zero error
    pub fn division_by_zero() -> Self {
        Self::DivisionByZero
    }

    /// Create an invalid operation error
    pub fn invalid_operation(message: impl Into<String>) -> Self {
        Self::InvalidOperation {
            message: message.into(),
        }
    }

    /// Create an invalid function arguments error
    pub fn invalid_function_args(func_name: impl Into<String>, expected: usize, actual: usize) -> Self {
        Self::ExpressionEvalError(
            format!("Function {} expects {} arguments, got {}", func_name.into(), expected, actual)
        )
    }

    /// Check if this error is fatal (should terminate the program)
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::ConnectionError(_)
                | Self::InitializationError(_)
                | Self::NoInitializationSteps
                | Self::ThreadPanic(_)
                | Self::UnexpectedStatus(_)
                | Self::UnexpectedNodeType(_)
        )
    }

    /// Check if this error is transient (can be retried)
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::ConnectionError(_) | Self::QueryError(_) | Self::ThreadError(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PgBenchError::DivisionByZero;
        assert_eq!(err.to_string(), "Division by zero");

        let err = PgBenchError::InvalidArgument("test".to_string());
        assert_eq!(err.to_string(), "Invalid argument: test");
    }

    #[test]
    fn test_query_with_context() {
        let err = PgBenchError::query_with_context("connection lost", "SELECT * FROM users");
        assert!(err.to_string().contains("connection lost"));
        assert!(err.to_string().contains("SELECT * FROM users"));
    }

    #[test]
    fn test_file_error() {
        let err = PgBenchError::file_error("open", "/tmp/test.sql", "permission denied");
        assert!(err.to_string().contains("open"));
        assert!(err.to_string().contains("/tmp/test.sql"));
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_script_error() {
        let err = PgBenchError::script_error("custom.sql", 42, "variable not found");
        assert!(err.to_string().contains("custom.sql"));
        assert!(err.to_string().contains("42"));
        assert!(err.to_string().contains("variable not found"));
    }

    #[test]
    fn test_malformed_variable() {
        let err = PgBenchError::malformed_variable("myvar", "invalid123");
        assert!(err.to_string().contains("myvar"));
        assert!(err.to_string().contains("invalid123"));
    }

    #[test]
    fn test_coercion_error() {
        let err = PgBenchError::coercion_error("string", "integer");
        assert!(err.to_string().contains("string"));
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn test_operation_overflow() {
        let err = PgBenchError::operation_overflow("bigint add");
        assert!(err.to_string().contains("bigint add"));
        assert!(err.to_string().contains("overflow"));
    }

    #[test]
    fn test_is_fatal() {
        assert!(PgBenchError::ConnectionError("test".to_string()).is_fatal());
        assert!(PgBenchError::NoInitializationSteps.is_fatal());
        assert!(PgBenchError::UnexpectedStatus(1).is_fatal());

        assert!(!PgBenchError::DivisionByZero.is_fatal());
        assert!(!PgBenchError::InvalidArgument("test".to_string()).is_fatal());
    }

    #[test]
    fn test_is_transient() {
        assert!(PgBenchError::ConnectionError("test".to_string()).is_transient());
        assert!(PgBenchError::QueryError("test".to_string()).is_transient());
        assert!(PgBenchError::ThreadError("test".to_string()).is_transient());

        assert!(!PgBenchError::DivisionByZero.is_transient());
        assert!(!PgBenchError::InvalidArgument("test".to_string()).is_transient());
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let pgbench_err: PgBenchError = io_err.into();
        assert!(matches!(pgbench_err, PgBenchError::IoError(_)));
    }

    #[test]
    fn test_from_parse_int_error() {
        let parse_err = "not_a_number".parse::<i32>().unwrap_err();
        let pgbench_err: PgBenchError = parse_err.into();
        assert!(matches!(
            pgbench_err,
            PgBenchError::InvalidInputSyntax { .. }
        ));
    }

    #[test]
    fn test_from_parse_float_error() {
        let parse_err = "not_a_float".parse::<f64>().unwrap_err();
        let pgbench_err: PgBenchError = parse_err.into();
        assert!(matches!(
            pgbench_err,
            PgBenchError::InvalidInputSyntax { .. }
        ));
    }

    #[test]
    fn test_error_propagation() {
        fn might_fail() -> PgBenchResult<i32> {
            Err(PgBenchError::DivisionByZero)
        }

        fn calls_might_fail() -> PgBenchResult<i32> {
            might_fail()?;
            Ok(42)
        }

        let result = calls_might_fail();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PgBenchError::DivisionByZero));
    }

    #[test]
    fn test_error_chain() {
        fn inner() -> PgBenchResult<()> {
            Err(PgBenchError::InvalidArgument("bad value".to_string()))
        }

        fn outer() -> PgBenchResult<()> {
            inner().map_err(|e| PgBenchError::ConfigError(format!("Config failed: {}", e)))?;
            Ok(())
        }

        let result = outer();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PgBenchError::ConfigError(_)));
        assert!(err.to_string().contains("bad value"));
    }

    #[test]
    fn test_type_mismatch_error() {
        let err = PgBenchError::TypeMismatch {
            expected: "integer".to_string(),
            actual: "string".to_string(),
        };
        assert!(err.to_string().contains("expected integer"));
        assert!(err.to_string().contains("got string"));
    }

    #[test]
    fn test_value_out_of_range() {
        let err = PgBenchError::ValueOutOfRange {
            value: "9999999999999".to_string(),
            type_name: "bigint".to_string(),
        };
        assert!(err.to_string().contains("9999999999999"));
        assert!(err.to_string().contains("bigint"));
    }

    #[test]
    fn test_empty_range_error() {
        let err = PgBenchError::EmptyRange;
        assert!(err.to_string().contains("Empty range"));
    }

    #[test]
    fn test_too_many_scripts() {
        let err = PgBenchError::TooManyScripts { max: 256 };
        assert!(err.to_string().contains("256"));
    }

    #[test]
    fn test_too_many_function_args() {
        let err = PgBenchError::TooManyFunctionArgs { max: 16 };
        assert!(err.to_string().contains("16"));
    }
}
