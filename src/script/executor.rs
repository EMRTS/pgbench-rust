//! Script executor for pgbench transaction scripts
//!
//! This module executes parsed commands (SQL and meta-commands) within the context
//! of a client connection.
//!
//! Reference: original-source/pgbench.c executeStatement() (lines 5540-5760)

use crate::error::{PgBenchError, PgBenchResult};
use crate::expr::eval::{evaluate_expression, EvalContext};
use crate::expr::parser::parse_expression;
use crate::types::{Command, MetaCommand, PgBenchExpr, PgBenchValue};
use crate::worker::state::ClientState;
use std::time::Duration;

/// Script executor
///
/// Executes commands from a parsed script in the context of a client state.
/// Handles both SQL commands and meta-commands (\set, \sleep, \if, etc.)
pub struct ScriptExecutor;

impl ScriptExecutor {
    /// Execute a single command in the context of a client (async)
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
    pub async fn execute(command: &Command, client: &mut ClientState) -> PgBenchResult<bool> {
        match command {
            Command::Sql { query } => Self::execute_sql(query, client).await,
            Command::Meta(meta) => Self::execute_meta(meta, client).await,
        }
    }

    /// Execute a SQL command (async)
    ///
    /// Substitutes variables in the SQL string before execution.
    /// Reference: pgbench.c executeStatement() SQL case (lines 5545-5640)
    async fn execute_sql(sql: &str, client: &mut ClientState) -> PgBenchResult<bool> {
        log::debug!("Client {}: Executing SQL: {}", client.id, sql);

        // Perform variable substitution
        let substituted_sql = Self::substitute_variables(sql, client)?;

        log::debug!("Client {}: After substitution: {}", client.id, substituted_sql);

        // Execute the query (async)
        client
            .executor
            .execute(&substituted_sql)
            .await
            .map_err(|e| {
                PgBenchError::QueryError(format!(
                    "SQL execution failed for query '{}': {}",
                    substituted_sql, e
                ))
            })?;

        Ok(true)
    }

    /// Substitute variables in SQL query
    ///
    /// Replaces :varname with the actual value from client variables
    /// Reference: pgbench.c replaceVariable() lines 5312-5378
    fn substitute_variables(sql: &str, client: &ClientState) -> PgBenchResult<String> {
        use regex::Regex;

        // Match :varname pattern (alphanumeric and underscore)
        let re = Regex::new(r":([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        let mut result = sql.to_string();

        // Find all variable references and replace them
        for cap in re.captures_iter(sql) {
            let var_name = &cap[1];
            let placeholder = &cap[0]; // :varname

            // Look up variable value
            let value = client.get_variable(var_name)
                .ok_or_else(|| {
                    PgBenchError::VariableNotFound(format!(
                        "Variable '{}' not found in SQL: {}",
                        var_name, sql
                    ))
                })?;

            // Convert value to SQL literal
            let sql_literal = match &value.value {
                crate::types::PgBenchValueData::Int(i) => i.to_string(),
                crate::types::PgBenchValueData::Double(d) => d.to_string(),
                crate::types::PgBenchValueData::Boolean(b) => b.to_string(),
                crate::types::PgBenchValueData::Null => "NULL".to_string(),
                crate::types::PgBenchValueData::NoValue => {
                    return Err(PgBenchError::ExpressionEvalError(
                        format!("Variable '{}' has no value", var_name)
                    ))
                }
            };

            // Replace this occurrence
            result = result.replace(placeholder, &sql_literal);
        }

        Ok(result)
    }

    /// Execute a meta-command (async for consistency, though most don't await)
    ///
    /// Handles \set, \sleep, \if, \elif, \else, \endif, \setshell, \startpipeline, \endpipeline
    /// Reference: pgbench.c executeStatement() meta-command cases
    async fn execute_meta(meta: &MetaCommand, client: &mut ClientState) -> PgBenchResult<bool> {
        match meta {
            MetaCommand::Set { variable, value } => Self::execute_set(variable, value, client).await,
            MetaCommand::Sleep { duration } => Self::execute_sleep(*duration, client).await,
            MetaCommand::SetShell { variable, command } => {
                Self::execute_setshell(variable, command, client).await
            }
            MetaCommand::If { condition } => Self::execute_if(condition, client).await,
            MetaCommand::ElseIf { condition } => Self::execute_elif(condition, client).await,
            MetaCommand::Else => Self::execute_else(client).await,
            MetaCommand::EndIf => Self::execute_endif(client).await,
            MetaCommand::StartPipeline => Self::execute_start_pipeline(client).await,
            MetaCommand::EndPipeline => Self::execute_end_pipeline(client).await,
        }
    }

    /// Execute \set command (async for consistency)
    ///
    /// Parses and evaluates the expression, then sets the variable.
    /// Reference: pgbench.c executeStatement() META_SET case (lines 5650-5670)
    async fn execute_set(
        variable: &str,
        value_str: &str,
        client: &mut ClientState,
    ) -> PgBenchResult<bool> {
        log::debug!("Client {}: Setting variable: {} = {}", client.id, variable, value_str);

        // Parse the value string as an expression
        let expr = parse_expression(value_str)?;

        // Create evaluation context
        let mut context = EvalContext::new();

        // Add client variables to context
        for (name, val) in &client.variables {
            context.set_variable(name.clone(), val.clone());
        }

        // Evaluate the expression
        let value = evaluate_expression(&expr, &mut context, &mut client.func_rng)?;

        // Set the variable
        client.set_variable(variable.to_string(), value.clone());

        log::debug!(
            "Client {}: Variable {} = {}",
            client.id,
            variable,
            value
        );

        Ok(true)
    }

    /// Execute \sleep command
    ///
    /// Sets the client to sleep for the specified duration (in seconds).
    /// Reference: pgbench.c executeStatement() META_SLEEP case (lines 5675-5690)
    async fn execute_sleep(duration_sec: f64, client: &mut ClientState) -> PgBenchResult<bool> {
        // Convert seconds to microseconds
        let duration_us = (duration_sec * 1_000_000.0) as u64;
        let duration = Duration::from_micros(duration_us);

        log::debug!(
            "Client {}: Sleeping for {:?} ({} seconds)",
            client.id,
            duration,
            duration_sec
        );

        // Set client to sleep (state machine will handle the actual sleep)
        client.sleep_for(duration);

        Ok(true)
    }

    /// Execute \setshell command
    ///
    /// Runs a shell command and sets the variable to its output.
    /// Reference: pgbench.c executeStatement() META_SETSHELL case (lines 5695-5720)
    async fn execute_setshell(
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
                PgBenchError::ScriptExecutionError {
                    script: "unknown".to_string(),
                    command: 0,
                    message: format!("Failed to execute shell command: {}", e),
                }
            })?;

        // Get stdout as string (trim both leading and trailing whitespace)
        let output_str = String::from_utf8_lossy(&output.stdout);
        let output_str = output_str.trim();

        // Try to parse as integer first, then as double, otherwise use as string
        let value = if let Ok(i) = output_str.parse::<i64>() {
            PgBenchValue::int(i)
        } else if let Ok(d) = output_str.parse::<f64>() {
            PgBenchValue::double(d)
        } else {
            // pgbench doesn't support string values, so we'll try to parse as integer
            // If that fails, set to NULL
            log::warn!(
                "Client {}: Shell command output '{}' is not a valid number, setting {} to NULL",
                client.id,
                output_str,
                variable
            );
            PgBenchValue::null()
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
    async fn execute_if(condition: &PgBenchExpr, client: &mut ClientState) -> PgBenchResult<bool> {
        log::debug!("Client {}: Evaluating \\if condition", client.id);

        // Create evaluation context
        let mut context = EvalContext::new();

        // Add client variables to context
        for (name, val) in &client.variables {
            context.set_variable(name.clone(), val.clone());
        }

        // Evaluate condition as boolean
        let value = evaluate_expression(condition, &mut context, &mut client.func_rng)?;
        let result = value.coerce_to_bool()?;

        log::debug!("Client {}: \\if condition = {}", client.id, result);

        // Return whether to continue (true = execute following commands, false = skip)
        Ok(result)
    }

    /// Execute \elif command
    ///
    /// Similar to \if but used after a false \if
    /// Reference: pgbench.c executeStatement() META_ELIF case
    async fn execute_elif(condition: &PgBenchExpr, client: &mut ClientState) -> PgBenchResult<bool> {
        log::debug!("Client {}: Evaluating \\elif condition", client.id);

        // Create evaluation context
        let mut context = EvalContext::new();

        // Add client variables to context
        for (name, val) in &client.variables {
            context.set_variable(name.clone(), val.clone());
        }

        // Evaluate condition as boolean
        let value = evaluate_expression(condition, &mut context, &mut client.func_rng)?;
        let result = value.coerce_to_bool()?;

        log::debug!("Client {}: \\elif condition = {}", client.id, result);

        Ok(result)
    }

    /// Execute \else command
    ///
    /// Switches to executing commands after a false \if/\elif
    /// Reference: pgbench.c executeStatement() META_ELSE case
    async fn execute_else(_client: &mut ClientState) -> PgBenchResult<bool> {
        // \else always returns true (execute following commands)
        Ok(true)
    }

    /// Execute \endif command
    ///
    /// Ends a conditional block
    /// Reference: pgbench.c executeStatement() META_ENDIF case
    async fn execute_endif(_client: &mut ClientState) -> PgBenchResult<bool> {
        // \endif always returns true (continue execution)
        Ok(true)
    }

    /// Execute \startpipeline command
    ///
    /// Starts PostgreSQL pipeline mode (PostgreSQL 14+)
    /// Reference: pgbench.c executeStatement() META_STARTPIPELINE case
    async fn execute_start_pipeline(_client: &mut ClientState) -> PgBenchResult<bool> {
        // TODO: Implement pipeline mode when postgres crate supports it
        // For now, this is a no-op
        log::debug!("\\startpipeline: Pipeline mode not yet implemented");
        Ok(true)
    }

    /// Execute \endpipeline command
    ///
    /// Ends PostgreSQL pipeline mode
    /// Reference: pgbench.c executeStatement() META_ENDPIPELINE case
    async fn execute_end_pipeline(_client: &mut ClientState) -> PgBenchResult<bool> {
        // TODO: Implement pipeline mode when postgres crate supports it
        // For now, this is a no-op
        log::debug!("\\endpipeline: Pipeline mode not yet implemented");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let trimmed = output.trim();
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
