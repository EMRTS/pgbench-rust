//! Utility functions for pgbench

use crate::error::{PgBenchError, PgBenchResult};
use std::fs;
use std::path::Path;

/// Parse a 64-bit integer from a string, matching C strtoint64 behavior
///
/// This function mimics the behavior of strtoint64 from the C implementation,
/// handling leading/trailing whitespace and proper overflow detection.
pub fn parse_int64(s: &str, error_ok: bool) -> PgBenchResult<i64> {
    let trimmed = s.trim();

    match trimmed.parse::<i64>() {
        Ok(val) => Ok(val),
        Err(e) => {
            if error_ok {
                Err(PgBenchError::InvalidInputSyntax {
                    type_name: "bigint".to_string(),
                    value: s.to_string(),
                })
            } else {
                // Check if it's an overflow or invalid syntax
                if trimmed.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '+') {
                    Err(PgBenchError::ValueOutOfRange {
                        value: s.to_string(),
                        type_name: "bigint".to_string(),
                    })
                } else {
                    Err(PgBenchError::InvalidInputSyntax {
                        type_name: "bigint".to_string(),
                        value: format!("{}: {}", s, e),
                    })
                }
            }
        }
    }
}

/// Parse a double from a string, matching C strtodouble behavior
///
/// Handles special values (inf, nan) and proper error reporting.
pub fn parse_double(s: &str, error_ok: bool) -> PgBenchResult<f64> {
    let trimmed = s.trim();

    match trimmed.parse::<f64>() {
        Ok(val) => {
            // Check for out of range (infinity)
            if val.is_infinite() && !trimmed.to_lowercase().contains("inf") {
                if error_ok {
                    Err(PgBenchError::ValueOutOfRange {
                        value: s.to_string(),
                        type_name: "double".to_string(),
                    })
                } else {
                    Err(PgBenchError::ValueOutOfRange {
                        value: s.to_string(),
                        type_name: "double".to_string(),
                    })
                }
            } else {
                Ok(val)
            }
        }
        Err(e) => {
            if error_ok {
                Err(PgBenchError::InvalidInputSyntax {
                    type_name: "double".to_string(),
                    value: s.to_string(),
                })
            } else {
                Err(PgBenchError::InvalidInputSyntax {
                    type_name: "double".to_string(),
                    value: format!("{}: {}", s, e),
                })
            }
        }
    }
}

/// Check if a variable name is valid
///
/// Valid names contain ASCII letters, digits, underscores, or non-ASCII characters.
/// Must not be empty and must start with a letter or underscore.
pub fn is_valid_variable_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();

    // First character must be letter, underscore, or non-ASCII
    if let Some(first) = chars.next() {
        if first.is_ascii() {
            // ASCII character must be letter or underscore
            if !first.is_ascii_alphabetic() && first != '_' {
                return false;
            }
        }
        // Non-ASCII is allowed as first character
    }

    // Remaining characters must be letters, digits, underscores, or non-ASCII
    for c in chars {
        if c.is_ascii() {
            // ASCII character must be alphanumeric or underscore
            if !c.is_ascii_alphanumeric() && c != '_' {
                return false;
            }
        }
        // Non-ASCII is allowed
    }

    true
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

/// Format throughput (transactions per second)
pub fn format_tps(transactions: u64, duration_micros: u64) -> String {
    if duration_micros == 0 {
        return "N/A".to_string();
    }

    let duration_secs = duration_micros as f64 / 1_000_000.0;
    let tps = transactions as f64 / duration_secs;

    format!("{:.2}", tps)
}

/// Format latency (microseconds to milliseconds)
pub fn format_latency(micros: u64) -> String {
    let ms = micros as f64 / 1_000.0;
    format!("{:.3} ms", ms)
}

/// Calculate percentile from sorted array of latencies
pub fn calculate_percentile(sorted_latencies: &[u64], percentile: f64) -> u64 {
    if sorted_latencies.is_empty() {
        return 0;
    }

    let len = sorted_latencies.len();
    // Use truncation (as usize) instead of rounding for percentile calculation
    let index = ((percentile / 100.0) * (len - 1) as f64) as usize;
    let index = index.min(len - 1);
    sorted_latencies[index]
}

/// Read entire file to string
pub fn read_file_to_string(path: &Path) -> PgBenchResult<String> {
    fs::read_to_string(path).map_err(|e| {
        PgBenchError::file_error("read", path.display().to_string(), e.to_string())
    })
}

/// Check if file exists
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Get file size in bytes
pub fn get_file_size(path: &Path) -> PgBenchResult<u64> {
    let metadata = fs::metadata(path).map_err(|e| {
        PgBenchError::file_error("stat", path.display().to_string(), e.to_string())
    })?;

    Ok(metadata.len())
}

/// Trim and normalize whitespace in string
pub fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Check if string contains only digits (and optional sign)
pub fn is_integer_string(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut chars = trimmed.chars();

    // Check for optional sign
    if let Some(first) = chars.next() {
        if first == '-' || first == '+' {
            // Must have at least one digit after sign
            if let Some(second) = chars.next() {
                if !second.is_ascii_digit() {
                    return false;
                }
            } else {
                return false;
            }
        } else if !first.is_ascii_digit() {
            return false;
        }
    }

    // Rest must be digits
    chars.all(|c| c.is_ascii_digit())
}

/// Check if a value is within a range (inclusive)
pub fn check_range_i64(value: i64, min: i64, max: i64, name: &str) -> PgBenchResult<()> {
    if value < min || value > max {
        Err(PgBenchError::InvalidArgument(format!(
            "{} must be between {} and {}, got {}",
            name, min, max, value
        )))
    } else {
        Ok(())
    }
}

/// Check if a value is within a range (inclusive)
pub fn check_range_f64(value: f64, min: f64, max: f64, name: &str) -> PgBenchResult<()> {
    if value < min || value > max {
        Err(PgBenchError::InvalidArgument(format!(
            "{} must be between {} and {}, got {}",
            name, min, max, value
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_int64() {
        assert_eq!(parse_int64("42", false).unwrap(), 42);
        assert_eq!(parse_int64("-123", false).unwrap(), -123);
        assert_eq!(parse_int64("  456  ", false).unwrap(), 456);
        assert_eq!(parse_int64("+789", false).unwrap(), 789);
        assert!(parse_int64("not a number", false).is_err());
        assert!(parse_int64("", false).is_err());
    }

    #[test]
    fn test_parse_int64_overflow() {
        // Test overflow detection
        let overflow = "99999999999999999999999999999";
        let result = parse_int64(overflow, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_double() {
        assert_eq!(parse_double("3.14", false).unwrap(), 3.14);
        assert_eq!(parse_double("-2.5", false).unwrap(), -2.5);
        assert_eq!(parse_double("  1.23  ", false).unwrap(), 1.23);
        assert_eq!(parse_double("inf", false).unwrap(), f64::INFINITY);
        assert!(parse_double("nan", false).unwrap().is_nan());
        assert!(parse_double("not a number", false).is_err());
    }

    #[test]
    fn test_is_valid_variable_name() {
        // Valid names
        assert!(is_valid_variable_name("myvar"));
        assert!(is_valid_variable_name("_private"));
        assert!(is_valid_variable_name("var123"));
        assert!(is_valid_variable_name("CamelCase"));

        // Invalid names
        assert!(!is_valid_variable_name(""));
        assert!(!is_valid_variable_name("123var")); // starts with digit
        assert!(!is_valid_variable_name("my-var")); // contains hyphen
        assert!(!is_valid_variable_name("my var")); // contains space
        assert!(!is_valid_variable_name("my.var")); // contains dot
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500 μs");
        assert_eq!(format_duration(1_500), "1.50 ms");
        assert_eq!(format_duration(2_500_000), "2.50 s");
        assert_eq!(format_duration(125_000_000), "2 min 5 s");
        assert_eq!(format_duration(0), "0 μs");
    }

    #[test]
    fn test_format_tps() {
        assert_eq!(format_tps(1000, 1_000_000), "1000.00");
        assert_eq!(format_tps(500, 2_000_000), "250.00");
        assert_eq!(format_tps(0, 1_000_000), "0.00");
        assert_eq!(format_tps(100, 0), "N/A");
    }

    #[test]
    fn test_format_latency() {
        assert_eq!(format_latency(1000), "1.000 ms");
        assert_eq!(format_latency(2500), "2.500 ms");
        assert_eq!(format_latency(100), "0.100 ms");
    }

    #[test]
    fn test_calculate_percentile() {
        let latencies = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

        assert_eq!(calculate_percentile(&latencies, 0.0), 10);
        assert_eq!(calculate_percentile(&latencies, 50.0), 50);
        assert_eq!(calculate_percentile(&latencies, 90.0), 90);
        assert_eq!(calculate_percentile(&latencies, 100.0), 100);

        // Empty array
        assert_eq!(calculate_percentile(&[], 50.0), 0);
    }

    #[test]
    fn test_read_file_to_string() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test content").unwrap();

        let content = read_file_to_string(temp_file.path()).unwrap();
        assert_eq!(content.trim(), "test content");
    }

    #[test]
    fn test_file_exists() {
        let temp_file = NamedTempFile::new().unwrap();
        assert!(file_exists(temp_file.path()));

        let non_existent = Path::new("/nonexistent/file.txt");
        assert!(!file_exists(non_existent));
    }

    #[test]
    fn test_get_file_size() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "hello").unwrap();
        temp_file.flush().unwrap();

        let size = get_file_size(temp_file.path()).unwrap();
        assert_eq!(size, 6); // "hello\n" = 6 bytes
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("  hello   world  "), "hello world");
        assert_eq!(normalize_whitespace("a\tb\nc"), "a b c");
        assert_eq!(normalize_whitespace("   "), "");
        assert_eq!(normalize_whitespace("single"), "single");
    }

    #[test]
    fn test_is_integer_string() {
        assert!(is_integer_string("123"));
        assert!(is_integer_string("-456"));
        assert!(is_integer_string("+789"));
        assert!(is_integer_string("  42  "));

        assert!(!is_integer_string(""));
        assert!(!is_integer_string("12.34"));
        assert!(!is_integer_string("abc"));
        assert!(!is_integer_string("12a"));
        assert!(!is_integer_string("-"));
        assert!(!is_integer_string("+"));
    }

    #[test]
    fn test_check_range_i64() {
        assert!(check_range_i64(5, 0, 10, "value").is_ok());
        assert!(check_range_i64(0, 0, 10, "value").is_ok());
        assert!(check_range_i64(10, 0, 10, "value").is_ok());

        assert!(check_range_i64(-1, 0, 10, "value").is_err());
        assert!(check_range_i64(11, 0, 10, "value").is_err());
    }

    #[test]
    fn test_check_range_f64() {
        assert!(check_range_f64(5.0, 0.0, 10.0, "value").is_ok());
        assert!(check_range_f64(0.0, 0.0, 10.0, "value").is_ok());
        assert!(check_range_f64(10.0, 0.0, 10.0, "value").is_ok());

        assert!(check_range_f64(-0.1, 0.0, 10.0, "value").is_err());
        assert!(check_range_f64(10.1, 0.0, 10.0, "value").is_err());
    }

    #[test]
    fn test_get_username() {
        let username = get_username();
        assert!(!username.is_empty());
    }
}
