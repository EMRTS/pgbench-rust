//! Database connection management

use crate::error::{PgBenchError, PgBenchResult};
use postgres::{Client, NoTls};

/// Wrapper around PostgreSQL connection
pub struct PgBenchConnection {
    client: Client,
}

impl PgBenchConnection {
    /// Create a new database connection
    pub fn connect(connection_string: &str) -> PgBenchResult<Self> {
        let client = Client::connect(connection_string, NoTls)
            .map_err(|e| PgBenchError::ConnectionError(e.to_string()))?;

        Ok(Self { client })
    }

    /// Get a reference to the underlying client
    pub fn client(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Execute a simple query
    pub fn execute(&mut self, query: &str) -> PgBenchResult<u64> {
        self.client
            .execute(query, &[])
            .map_err(|e| PgBenchError::QueryError(e.to_string()))
    }

    /// Test if connection is alive
    pub fn is_alive(&mut self) -> bool {
        self.client.execute("SELECT 1", &[]).is_ok()
    }
}

// TODO: Implement connection pooling for multi-threaded execution
// TODO: Add prepared statement support
// TODO: Add async query execution support
