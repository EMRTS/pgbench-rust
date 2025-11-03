//! Utility functions for pgbench

use crate::error::{PgBenchError, PgBenchResult};

/// Parse a 64-bit integer from a string
pub fn parse_int64(s: &str, error_ok: bool) -> PgBenchResult<i64> {
    s.parse::<i64>().map_err(|e| {
        if error_ok {
            PgBenchError::InvalidArgument(format!("Invalid integer: {}", s))
        } else {
            PgBenchError::InvalidArgument(format!("Invalid integer '{}': {}", s, e))
        }
    })
}

/// Parse a double from a string
pub fn parse_double(s: &str, error_ok: bool) -> PgBenchResult<f64> {
    s.parse::<f64>().map_err(|e| {
        if error_ok {
            PgBenchError::InvalidArgument(format!("Invalid double: {}", s))
        } else {
            PgBenchError::InvalidArgument(format!("Invalid double '{}': {}", s, e))
        }
    })
}

/// Get current username
pub fn get_username() -> String {
    whoami::username()
}

/// Format duration in human-readable form
pub fn format_duration(micros: u64) -> String {
    if micros < 1_000 {
        format!("{} μs", micros)
    } else if micros < 1_000_000 {
        format!("{:.2} ms", micros as f64 / 1_000.0)
    } else if micros < 60_000_000 {
        format!("{:.2} s", micros as f64 / 1_000_000.0)
    } else {
        let seconds = micros / 1_000_000;
        let minutes = seconds / 60;
        let remaining_seconds = seconds % 60;
        format!("{} min {} s", minutes, remaining_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int64() {
        assert_eq!(parse_int64("42", false).unwrap(), 42);
        assert_eq!(parse_int64("-123", false).unwrap(), -123);
        assert!(parse_int64("not a number", false).is_err());
    }

    #[test]
    fn test_parse_double() {
        assert_eq!(parse_double("3.14", false).unwrap(), 3.14);
        assert_eq!(parse_double("-2.5", false).unwrap(), -2.5);
        assert!(parse_double("not a number", false).is_err());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500 μs");
        assert_eq!(format_duration(1_500), "1.50 ms");
        assert_eq!(format_duration(2_500_000), "2.50 s");
        assert_eq!(format_duration(125_000_000), "2 min 5 s");
    }
}
