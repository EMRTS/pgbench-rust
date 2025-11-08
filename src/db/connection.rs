//! Database connection management
//!
//! Migrated to tokio-postgres for async operations (Phase 9.2)
//! This fixes the multi-client single-thread deadlock issue.

use crate::error::{PgBenchError, PgBenchResult};
use log::{debug, warn};
use tokio_postgres::{Client, NoTls};
use std::time::Duration;

/// Maximum number of connection retry attempts
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Delay between retry attempts (in milliseconds)
const RETRY_DELAY_MS: u64 = 1000;

/// Wrapper around PostgreSQL connection
pub struct PgBenchConnection {
    client: Client,
    connection_string: String,
}

impl std::fmt::Debug for PgBenchConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgBenchConnection")
            .field("connection_string", &Self::sanitize_connection_string(&self.connection_string))
            .finish()
    }
}

impl PgBenchConnection {
    /// Create a new database connection with retry logic (async)
    ///
    /// # Arguments
    /// * `connection_string` - PostgreSQL connection string (e.g., "postgres://localhost/dbname")
    ///
    /// # Returns
    /// * `Ok(PgBenchConnection)` on successful connection
    /// * `Err(PgBenchError)` if connection fails after all retries
    ///
    /// # Examples
    /// ```no_run
    /// use pgbench_rust::db::connection::PgBenchConnection;
    ///
    /// # async fn example() -> Result<(), pgbench_rust::error::PgBenchError> {
    /// let conn = PgBenchConnection::connect("postgres://localhost/test").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(connection_string: &str) -> PgBenchResult<Self> {
        // Validate connection string is not empty
        if connection_string.trim().is_empty() {
            return Err(PgBenchError::ConnectionError(
                "Connection string cannot be empty".to_string(),
            ));
        }

        debug!("Attempting to connect to database: {}",
               Self::sanitize_connection_string(connection_string));

        Self::connect_with_retry(connection_string, MAX_RETRY_ATTEMPTS).await
    }

    /// Connect with retry logic for transient failures (async)
    async fn connect_with_retry(connection_string: &str, max_attempts: u32) -> PgBenchResult<Self> {
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            match tokio_postgres::connect(connection_string, NoTls).await {
                Ok((client, connection)) => {
                    // Spawn the connection task to run in the background
                    tokio::spawn(async move {
                        if let Err(e) = connection.await {
                            log::error!("Database connection error: {}", e);
                        }
                    });

                    debug!("Successfully connected to database");

                    // Validate the connection is actually working
                    let mut conn = Self {
                        client,
                        connection_string: connection_string.to_string(),
                    };

                    if let Err(e) = conn.validate_connection().await {
                        warn!("Connection validation failed: {}", e);
                        last_error = Some(e);
                        continue;
                    }

                    return Ok(conn);
                }
                Err(e) => {
                    warn!(
                        "Connection attempt {}/{} failed: {}",
                        attempt, max_attempts, e
                    );
                    last_error = Some(PgBenchError::ConnectionError(e.to_string()));

                    // Don't sleep after the last attempt
                    if attempt < max_attempts {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            PgBenchError::ConnectionError("Connection failed for unknown reason".to_string())
        }))
    }

    /// Validate the connection by executing a simple query (async)
    async fn validate_connection(&mut self) -> PgBenchResult<()> {
        self.client
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| PgBenchError::ConnectionError(format!("Connection validation failed: {}", e)))?;
        Ok(())
    }

    /// Sanitize connection string for logging (remove password)
    fn sanitize_connection_string(conn_str: &str) -> String {
        // Simple approach: replace password in connection string
        // postgres://user:password@host/db -> postgres://user:***@host/db
        if let Some(at_pos) = conn_str.find('@') {
            if let Some(colon_pos) = conn_str[..at_pos].rfind(':') {
                let mut sanitized = conn_str.to_string();
                if let Some(protocol_end) = conn_str.find("//") {
                    let start = colon_pos + 1;
                    if start < at_pos && start > protocol_end {
                        sanitized.replace_range(start..at_pos, "***");
                        return sanitized;
                    }
                }
            }
        }
        conn_str.to_string()
    }

    /// Get a reference to the underlying client
    pub fn client(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Execute a query with parameters (async)
    ///
    /// # Arguments
    /// * `query` - SQL query string
    /// * `params` - Query parameters
    ///
    /// # Returns
    /// Number of rows affected
    pub async fn execute(&mut self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> PgBenchResult<u64> {
        self.client
            .execute(query, params)
            .await
            .map_err(|e| PgBenchError::QueryError(e.to_string()))
    }

    /// Start a COPY IN operation (async)
    ///
    /// Returns a sink that can be used to stream data to the database.
    ///
    /// # Example
    /// ```no_run
    /// use tokio::io::AsyncWriteExt;
    /// # use pgbench_rust::db::connection::PgBenchConnection;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut conn = PgBenchConnection::connect("postgres://localhost/test").await?;
    /// let sink = conn.copy_in("COPY my_table FROM STDIN").await?;
    /// // Use the sink with tokio::io::copy or similar
    /// # Ok(())
    /// # }
    /// ```
    pub async fn copy_in(&mut self, query: &str) -> PgBenchResult<tokio_postgres::CopyInSink<bytes::Bytes>> {
        let sink = self.client
            .copy_in(query)
            .await
            .map_err(|e| PgBenchError::QueryError(format!("COPY IN failed: {}", e)))?;
        Ok(sink)
    }

    /// Execute a query and return results (async)
    pub async fn query(&mut self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> PgBenchResult<Vec<tokio_postgres::Row>> {
        self.client
            .query(query, params)
            .await
            .map_err(|e| PgBenchError::QueryError(e.to_string()))
    }

    /// Test if connection is alive (async)
    ///
    /// # Returns
    /// `true` if connection is working, `false` otherwise
    pub async fn is_alive(&mut self) -> bool {
        self.client.execute("SELECT 1", &[]).await.is_ok()
    }

    /// Get the connection string (useful for debugging)
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Reconnect using the same connection string (async)
    pub async fn reconnect(&mut self) -> PgBenchResult<()> {
        debug!("Reconnecting to database");
        let new_conn = Self::connect(&self.connection_string).await?;
        self.client = new_conn.client;
        Ok(())
    }
}

// Note: CopyWriter removed - tokio-postgres uses CopyInSink (futures-based API)
// Users should work directly with tokio_postgres::CopyInSink returned by copy_in()

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_connection_string() {
        let result = PgBenchConnection::connect("").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PgBenchError::ConnectionError(_)));
    }

    #[test]
    fn test_sanitize_connection_string() {
        let conn_str = "postgres://user:secret123@localhost:5432/testdb";
        let sanitized = PgBenchConnection::sanitize_connection_string(conn_str);
        assert!(!sanitized.contains("secret123"));
        assert!(sanitized.contains("***"));
        assert!(sanitized.contains("localhost"));
    }

    #[test]
    fn test_sanitize_connection_string_no_password() {
        let conn_str = "postgres://localhost/testdb";
        let sanitized = PgBenchConnection::sanitize_connection_string(conn_str);
        assert_eq!(conn_str, sanitized);
    }

    // Integration tests (require PostgreSQL running)
    // Mark with #[ignore] to skip in normal test runs
    #[tokio::test]
    #[ignore]
    async fn test_real_connection() {
        // This requires a PostgreSQL instance running at localhost
        let result = PgBenchConnection::connect("postgres://localhost/postgres").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_connection_validation() {
        let mut conn = PgBenchConnection::connect("postgres://localhost/postgres")
            .await
            .expect("Failed to connect");
        assert!(conn.is_alive().await);
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_query() {
        let mut conn = PgBenchConnection::connect("postgres://localhost/postgres")
            .await
            .expect("Failed to connect");
        let result = conn.execute("SELECT 1", &[]).await;
        assert!(result.is_ok());
    }
}

// TODO: Implement connection pooling for multi-threaded execution
// TODO: Add prepared statement support
