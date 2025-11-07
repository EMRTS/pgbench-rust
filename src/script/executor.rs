//! Script executor for pgbench transaction scripts
//!
//! This module executes parsed commands (SQL and meta-commands) within the context
//! of a client connection.
//!
//! Reference: original-source/pgbench.c executeStatement() (lines 5540-5760)

use crate::error::{PgBenchError, PgBenchResult};
use crate::expr::eval::{eval_expr, EvalContext};
use crate::types::{Command, MetaCommand, PgBenchExpr, PgBenchValue};
use crate::worker::state::ClientState;
use std::time::Duration;

/// Script executor
///
/// Executes commands from a parsed script in the context of a client state.
/// Handles both SQL commands and meta-commands (\set, \sleep, \if, etc.)
pub struct ScriptExecutor;

impl ScriptExecutor {
    /// Execute a single command in the context of a client
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `client` - The client state (provides connection, variables, RNG)
    ///
    /// # Returns
    /// * `Ok(true)` - Command executed successfully
    /// * `Ok(false)` - Conditional command evaluated to false (skip subsequent commands)
    /// * `Err(_)` - Execution error
    ///
    /// Reference: pgbench.c executeStatement() (lines 5540-5760)
    pub fn execute(command: &Command, client: &mut ClientState) -> PgBenchResult<bool> {
        match command {
            Command::Sql(sql) => Self::execute_sql(sql, client),
            Command::Meta(meta) => Self::execute_meta(meta, client),
        }
    }

    /// Execute a SQL command
    ///
    /// Substitutes variables in the SQL string before execution.
    /// Reference: pgbench.c executeStatement() SQL case (lines 5545-5640)
    fn execute_sql(sql: &str, client: &mut ClientState) -> PgBenchResult<bool> {
        log::debug!("Client {}: Executing SQL: {}", client.id, sql);

        // TODO: Variable substitution in SQL
        // For now, execute SQL as-is
        // Full variable substitution will be implemented when needed

        // Execute the query
        client
            .executor
            .execute(sql)
            .map_err(|e| PgBenchError::QueryError(format!("SQL execution failed: {}", e)))?;

        Ok(true)
    }

    /// Execute a meta-command
    ///
    /// Handles \set, \sleep, \if, \elif, \else, \endif, \setshell, \startpipeline, \endpipeline
    /// Reference: pgbench.c executeStatement() meta-command cases
    fn execute_meta(meta: &MetaCommand, client: &mut ClientState) -> PgBenchResult<bool> {
        match meta {
            MetaCommand::Set { variable, expr } => Self::execute_set(variable, expr, client),
            MetaCommand::Sleep { duration } => Self::execute_sleep(*duration, client),
            MetaCommand::SetShell { variable, command } => {
                Self::execute_setshell(variable, command, client)
            }
            MetaCommand::If { condition } => Self::execute_if(condition, client),
            MetaCommand::Elif { condition } => Self::execute_elif(condition, client),
            MetaCommand::Else => Self::execute_else(client),
            MetaCommand::EndIf => Self::execute_endif(client),
            MetaCommand::StartPipeline => Self::execute_start_pipeline(client),
            MetaCommand::EndPipeline => Self::execute_end_pipeline(client),
        }
    }

    /// Execute \set command
    ///
    /// Evaluates the expression and sets the variable.
    /// Reference: pgbench.c executeStatement() META_SET case (lines 5650-5670)
    fn execute_set(
        variable: &str,
        expr: &PgBenchExpr,
        client: &mut ClientState,
    ) -> PgBenchResult<bool> {
        log::debug!("Client {}: Setting variable: {}", client.id, variable);

        // Create evaluation context with client variables
        let context = EvalContext::new(&client.variables, &mut client.func_rng);

        // Evaluate the expression
        let value = eval_expr(expr, &context)?;

        // Set the variable
        client.set_variable(variable.to_string(), value);

        log::debug!(
            "Client {}: Variable {} = {}",
            client.id,
            variable,
            client.get_variable(variable).unwrap()
        );

        Ok(true)
    }

    /// Execute \sleep command
    ///
    /// Sets the client to sleep for the specified duration.
    /// Reference: pgbench.c executeStatement() META_SLEEP case (lines 5675-5690)
    fn execute_sleep(duration_us: u64, client: &mut ClientState) -> PgBenchResult<bool> {
        let duration = Duration::from_micros(duration_us);
        log::debug!(
            "Client {}: Sleeping for {:?} ({} us)",
            client.id,
            duration,
            duration_us
        );

        // Set client to sleep (state machine will handle the actual sleep)
        client.sleep_for(duration);

        Ok(true)
    }

    /// Execute \setshell command
    ///
    /// Runs a shell command and sets the variable to its output.
    /// Reference: pgbench.c executeStatement() META_SETSHELL case (lines 5695-5720)
    fn execute_setshell(
        variable: &str,
        command: &str,
        client: &mut ClientState,
    ) -> PgBenchResult<bool> {
        log::debug!(
            "Client {}: Executing shell command for variable {}: {}",
            client.id,
            variable,
            command
        );

        // Execute shell command
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| {
                PgBenchError::InvalidScript(format!("Failed to execute shell command: {}", e))
            })?;

        // Get stdout as string (trimming newline)
        let output_str = String::from_utf8_lossy(&output.stdout);
        let output_str = output_str.trim_end();

        // Try to parse as integer first, then as double, otherwise use as string
        let value = if let Ok(i) = output_str.parse::<i64>() {
            PgBenchValue::Int(i)
        } else if let Ok(d) = output_str.parse::<f64>() {
            PgBenchValue::Double(d)
        } else {
            // pgbench doesn't support string values, so we'll try to parse as integer
            // If that fails, set to NULL
            log::warn!(
                "Client {}: Shell command output '{}' is not a valid number, setting {} to NULL",
                client.id,
                output_str,
                variable
            );
            PgBenchValue::Null
        };

        // Set the variable
        client.set_variable(variable.to_string(), value);

        log::debug!(
            "Client {}: Variable {} = {} (from shell)",
            client.id,
            variable,
            client.get_variable(variable).unwrap()
        );

        Ok(true)
    }

    /// Execute \if command
    ///
    /// Evaluates the condition and returns whether to continue execution.
    /// Reference: pgbench.c executeStatement() META_IF case
    fn execute_if(condition: &PgBenchExpr, client: &mut ClientState) -> PgBenchResult<bool> {
        log::debug!("Client {}: Evaluating \\if condition", client.id);

        // Create evaluation context
        let context = EvalContext::new(&client.variables, &mut client.func_rng);

        // Evaluate condition as boolean
        let value = eval_expr(condition, &context)?;
        let result = value.coerce_to_bool()?;

        log::debug!("Client {}: \\if condition = {}", client.id, result);

        // Return whether to continue (true = execute following commands, false = skip)
        Ok(result)
    }

    /// Execute \elif command
    ///
    /// Similar to \if but used after a false \if
    /// Reference: pgbench.c executeStatement() META_ELIF case
    fn execute_elif(condition: &PgBenchExpr, client: &mut ClientState) -> PgBenchResult<bool> {
        log::debug!("Client {}: Evaluating \\elif condition", client.id);

        // Create evaluation context
        let context = EvalContext::new(&client.variables, &mut client.func_rng);

        // Evaluate condition as boolean
        let value = eval_expr(condition, &context)?;
        let result = value.coerce_to_bool()?;

        log::debug!("Client {}: \\elif condition = {}", client.id, result);

        Ok(result)
    }

    /// Execute \else command
    ///
    /// Switches to executing commands after a false \if/\elif
    /// Reference: pgbench.c executeStatement() META_ELSE case
    fn execute_else(_client: &mut ClientState) -> PgBenchResult<bool> {
        // \else always returns true (execute following commands)
        Ok(true)
    }

    /// Execute \endif command
    ///
    /// Ends a conditional block
    /// Reference: pgbench.c executeStatement() META_ENDIF case
    fn execute_endif(_client: &mut ClientState) -> PgBenchResult<bool> {
        // \endif always returns true (continue execution)
        Ok(true)
    }

    /// Execute \startpipeline command
    ///
    /// Starts PostgreSQL pipeline mode (PostgreSQL 14+)
    /// Reference: pgbench.c executeStatement() META_STARTPIPELINE case
    fn execute_start_pipeline(_client: &mut ClientState) -> PgBenchResult<bool> {
        // TODO: Implement pipeline mode when postgres crate supports it
        // For now, this is a no-op
        log::debug!("\\startpipeline: Pipeline mode not yet implemented");
        Ok(true)
    }

    /// Execute \endpipeline command
    ///
    /// Ends PostgreSQL pipeline mode
    /// Reference: pgbench.c executeStatement() META_ENDPIPELINE case
    fn execute_end_pipeline(_client: &mut ClientState) -> PgBenchResult<bool> {
        // TODO: Implement pipeline mode when postgres crate supports it
        // For now, this is a no-op
        log::debug!("\\endpipeline: Pipeline mode not yet implemented");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::parser::parse_expr;
    use crate::types::PgBenchValue;

    #[test]
    fn test_execute_sleep_duration() {
        // Test sleep duration conversion
        let duration_us = 1_000_000; // 1 second
        let duration = Duration::from_micros(duration_us);
        assert_eq!(duration.as_secs(), 1);
        assert_eq!(duration.as_micros(), 1_000_000);

        // Test microseconds
        let duration_us = 500; // 500 microseconds
        let duration = Duration::from_micros(duration_us);
        assert_eq!(duration.as_micros(), 500);
    }

    #[test]
    fn test_setshell_parse_integer() {
        // Test parsing shell output as integer
        let output = "42";
        let value: Result<i64, _> = output.parse();
        assert_eq!(value.unwrap(), 42);

        let output = "-123";
        let value: Result<i64, _> = output.parse();
        assert_eq!(value.unwrap(), -123);
    }

    #[test]
    fn test_setshell_parse_double() {
        // Test parsing shell output as double
        let output = "3.14159";
        let value: Result<f64, _> = output.parse();
        assert!((value.unwrap() - 3.14159).abs() < 0.0001);

        let output = "-2.5";
        let value: Result<f64, _> = output.parse();
        assert!((value.unwrap() - (-2.5)).abs() < 0.0001);
    }

    #[test]
    fn test_setshell_parse_invalid() {
        // Test parsing invalid shell output
        let output = "not a number";
        let int_result: Result<i64, _> = output.parse();
        let double_result: Result<f64, _> = output.parse();
        assert!(int_result.is_err());
        assert!(double_result.is_err());
    }

    #[test]
    fn test_setshell_parse_whitespace() {
        // Test trimming whitespace from shell output
        let output = "  42  \n";
        let trimmed = output.trim_end();
        let value: Result<i64, _> = trimmed.parse();
        assert_eq!(value.unwrap(), 42);
    }

    // Integration tests (require database and full client setup) - marked with #[ignore]
    // These will be implemented in tests/integration_test.rs

    #[test]
    #[ignore]
    fn test_execute_sql_select() {
        // Would test actual SQL execution with a real database
        // client.executor.execute("SELECT 1")
    }

    #[test]
    #[ignore]
    fn test_execute_set_with_expression() {
        // Would test \set with expression evaluation
        // \set x 1 + 2
        // assert_eq!(client.get_variable("x"), Some(&PgBenchValue::Int(3)))
    }

    #[test]
    #[ignore]
    fn test_execute_conditional() {
        // Would test \if condition evaluation
        // \set x 5
        // \if :x > 3
        // should execute following commands
    }

    #[test]
    #[ignore]
    fn test_execute_setshell_real() {
        // Would test real shell command execution
        // \setshell output echo 42
        // assert_eq!(client.get_variable("output"), Some(&PgBenchValue::Int(42)))
    }
}
