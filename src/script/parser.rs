//! Script parser
//!
//! Parses pgbench transaction scripts into a sequence of SQL statements
//! and meta-commands.

use crate::error::{PgBenchError, PgBenchResult};
use crate::expr::parser::parse_expression;
use crate::types::{Command, MetaCommand};

/// Parse a script string into a list of commands
///
/// # Arguments
/// * `input` - The script text to parse
///
/// # Returns
/// A vector of parsed commands (SQL or meta-commands)
///
/// # Errors
/// Returns an error if the script contains invalid syntax
pub fn parse_script(input: &str) -> PgBenchResult<Vec<Command>> {
    let mut commands = Vec::new();
    let mut current_sql = String::new();
    let mut in_block_comment = false;

    for (line_no, line) in input.lines().enumerate() {
        let line_no = line_no + 1; // 1-indexed for error messages

        // Handle block comments
        if in_block_comment {
            if let Some(end_pos) = line.find("*/") {
                in_block_comment = false;
                // Continue processing after the block comment
                let remainder = &line[end_pos + 2..];
                if !remainder.trim().is_empty() {
                    // Process the remainder of the line recursively
                    parse_line(remainder, &mut commands, &mut current_sql, line_no)?;
                }
            }
            continue;
        }

        // Check for start of block comment
        if let Some(start_pos) = line.find("/*") {
            // Process everything before the comment
            let before_comment = &line[..start_pos];
            if !before_comment.trim().is_empty() {
                parse_line(before_comment, &mut commands, &mut current_sql, line_no)?;
            }

            // Check if comment ends on same line
            if let Some(end_pos) = line[start_pos..].find("*/") {
                let remainder = &line[start_pos + end_pos + 2..];
                if !remainder.trim().is_empty() {
                    parse_line(remainder, &mut commands, &mut current_sql, line_no)?;
                }
            } else {
                in_block_comment = true;
            }
            continue;
        }

        // Strip line comments (-- to end of line)
        let line = if let Some(pos) = line.find("--") {
            &line[..pos]
        } else {
            line
        };

        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        parse_line(trimmed, &mut commands, &mut current_sql, line_no)?;
    }

    // Flush any remaining SQL
    if !current_sql.trim().is_empty() {
        commands.push(Command::Sql {
            query: current_sql.trim().to_string(),
        });
    }

    Ok(commands)
}

/// Parse a single line
fn parse_line(
    line: &str,
    commands: &mut Vec<Command>,
    current_sql: &mut String,
    line_no: usize,
) -> PgBenchResult<()> {
    if line.starts_with('\\') {
        // This is a meta-command
        // First, flush any pending SQL
        if !current_sql.trim().is_empty() {
            commands.push(Command::Sql {
                query: current_sql.trim().to_string(),
            });
            current_sql.clear();
        }

        // Parse the meta-command
        let meta_cmd = parse_meta_command(&line[1..], line_no)?;
        commands.push(Command::Meta(meta_cmd));
    } else {
        // This is SQL - accumulate it, but split on semicolons
        if !current_sql.is_empty() {
            current_sql.push('\n');
        }
        current_sql.push_str(line);

        // Check if line ends with semicolon - if so, flush the SQL command
        if line.trim().ends_with(';') {
            commands.push(Command::Sql {
                query: current_sql.trim().to_string(),
            });
            current_sql.clear();
        }
    }

    Ok(())
}

/// Parse a meta-command (without the leading backslash)
fn parse_meta_command(line: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
    let cmd = parts[0].to_lowercase();
    let args = if parts.len() > 1 { parts[1].trim() } else { "" };

    match cmd.as_str() {
        "set" => parse_set_command(args, line_no),
        "setshell" => parse_setshell_command(args, line_no),
        "sleep" => parse_sleep_command(args, line_no),
        "if" => parse_if_command(args, line_no),
        "elif" => parse_elif_command(args, line_no),
        "else" => parse_else_command(args, line_no),
        "endif" => parse_endif_command(args, line_no),
        "startpipeline" => parse_startpipeline_command(args, line_no),
        "endpipeline" => parse_endpipeline_command(args, line_no),
        _ => Err(PgBenchError::ScriptParseError {
            message: format!("Unknown meta-command: \\{}", cmd),
            line: line_no,
        }),
    }
}

/// Parse \set variable expression
fn parse_set_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    let parts: Vec<&str> = args.splitn(2, char::is_whitespace).collect();

    if parts.is_empty() || parts[0].is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\set requires a variable name".to_string(),
            line: line_no,
        });
    }

    let variable = parts[0].to_string();
    let value = if parts.len() > 1 {
        parts[1].trim().to_string()
    } else {
        return Err(PgBenchError::ScriptParseError {
            message: "\\set requires a value expression".to_string(),
            line: line_no,
        });
    };

    Ok(MetaCommand::Set { variable, value })
}

/// Parse \setshell variable command
fn parse_setshell_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    let parts: Vec<&str> = args.splitn(2, char::is_whitespace).collect();

    if parts.is_empty() || parts[0].is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\setshell requires a variable name".to_string(),
            line: line_no,
        });
    }

    let variable = parts[0].to_string();
    let command = if parts.len() > 1 {
        parts[1].trim().to_string()
    } else {
        return Err(PgBenchError::ScriptParseError {
            message: "\\setshell requires a shell command".to_string(),
            line: line_no,
        });
    };

    Ok(MetaCommand::SetShell { variable, command })
}

/// Parse \sleep duration
fn parse_sleep_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    if args.is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\sleep requires a duration".to_string(),
            line: line_no,
        });
    }

    // Parse duration - can be a number with optional unit (us, ms, s)
    // Default unit is microseconds
    let args = args.trim();

    // Try to parse the number and unit
    let (num_str, unit) = if args.ends_with("us") {
        (&args[..args.len() - 2], 1.0)
    } else if args.ends_with("ms") {
        (&args[..args.len() - 2], 1000.0)
    } else if args.ends_with('s') {
        (&args[..args.len() - 1], 1_000_000.0)
    } else {
        (args, 1.0) // default is microseconds
    };

    let duration_value: f64 = num_str.trim().parse().map_err(|_| {
        PgBenchError::ScriptParseError {
            message: format!("Invalid sleep duration: {}", args),
            line: line_no,
        }
    })?;

    // Convert to microseconds
    let duration = duration_value * unit;

    Ok(MetaCommand::Sleep { duration })
}

/// Parse \if condition
fn parse_if_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    if args.is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\if requires a condition expression".to_string(),
            line: line_no,
        });
    }

    let condition = parse_expression(args).map_err(|e| PgBenchError::ScriptParseError {
        message: format!("Invalid \\if condition: {}", e),
        line: line_no,
    })?;

    Ok(MetaCommand::If { condition })
}

/// Parse \elif condition
fn parse_elif_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    if args.is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\elif requires a condition expression".to_string(),
            line: line_no,
        });
    }

    let condition = parse_expression(args).map_err(|e| PgBenchError::ScriptParseError {
        message: format!("Invalid \\elif condition: {}", e),
        line: line_no,
    })?;

    Ok(MetaCommand::ElseIf { condition })
}

/// Parse \else
fn parse_else_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    if !args.is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\else does not take any arguments".to_string(),
            line: line_no,
        });
    }

    Ok(MetaCommand::Else)
}

/// Parse \endif
fn parse_endif_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    if !args.is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\endif does not take any arguments".to_string(),
            line: line_no,
        });
    }

    Ok(MetaCommand::EndIf)
}

/// Parse \startpipeline
fn parse_startpipeline_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    if !args.is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\startpipeline does not take any arguments".to_string(),
            line: line_no,
        });
    }

    Ok(MetaCommand::StartPipeline)
}

/// Parse \endpipeline
fn parse_endpipeline_command(args: &str, line_no: usize) -> PgBenchResult<MetaCommand> {
    if !args.is_empty() {
        return Err(PgBenchError::ScriptParseError {
            message: "\\endpipeline does not take any arguments".to_string(),
            line: line_no,
        });
    }

    Ok(MetaCommand::EndPipeline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_sql() {
        let script = "SELECT 1;\nSELECT 2;";
        let commands = parse_script(script).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::Sql { query } => {
                assert!(query.contains("SELECT 1"));
                assert!(query.contains("SELECT 2"));
            }
            _ => panic!("Expected SQL command"),
        }
    }

    #[test]
    fn test_parse_set_command() {
        let script = "\\set x 42";
        let commands = parse_script(script).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::Meta(MetaCommand::Set { variable, value }) => {
                assert_eq!(variable, "x");
                assert_eq!(value, "42");
            }
            _ => panic!("Expected SET meta-command"),
        }
    }

    #[test]
    fn test_parse_sleep_command() {
        let script = "\\sleep 100ms";
        let commands = parse_script(script).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::Meta(MetaCommand::Sleep { duration }) => {
                assert_eq!(*duration, 100_000.0); // 100ms = 100,000 microseconds
            }
            _ => panic!("Expected SLEEP meta-command"),
        }
    }

    #[test]
    fn test_parse_if_command() {
        let script = "\\if 1 = 1";
        let commands = parse_script(script).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::Meta(MetaCommand::If { .. }) => {}
            _ => panic!("Expected IF meta-command"),
        }
    }

    #[test]
    fn test_parse_mixed_commands() {
        let script = "\\set aid random(1, 100000)\nSELECT * FROM accounts WHERE id = :aid;";
        let commands = parse_script(script).unwrap();
        assert_eq!(commands.len(), 2);

        match &commands[0] {
            Command::Meta(MetaCommand::Set { variable, value }) => {
                assert_eq!(variable, "aid");
                assert_eq!(value, "random(1, 100000)");
            }
            _ => panic!("Expected SET meta-command"),
        }

        match &commands[1] {
            Command::Sql { query } => {
                assert!(query.contains("SELECT"));
                assert!(query.contains(":aid"));
            }
            _ => panic!("Expected SQL command"),
        }
    }

    #[test]
    fn test_parse_comments() {
        let script = "-- This is a comment\nSELECT 1; -- inline comment\n/* block comment */\nSELECT 2;";
        let commands = parse_script(script).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            Command::Sql { query } => {
                assert!(query.contains("SELECT 1"));
                assert!(query.contains("SELECT 2"));
                assert!(!query.contains("comment"));
            }
            _ => panic!("Expected SQL command"),
        }
    }

    #[test]
    fn test_parse_builtin_tpcb_script() {
        use crate::script::builtin::BuiltinScript;

        let script = BuiltinScript::get("tpcb-like").unwrap();
        let commands = parse_script(script.script).unwrap();

        // Should have:
        // 4 \set commands
        // 1 BEGIN
        // 4 UPDATE/SELECT/INSERT commands
        // 1 END
        // Total: at least 10 commands (some SQL might be merged)
        assert!(commands.len() >= 6);

        // Check first command is \set
        match &commands[0] {
            Command::Meta(MetaCommand::Set { variable, .. }) => {
                assert_eq!(variable, "aid");
            }
            _ => panic!("Expected SET meta-command"),
        }
    }

    #[test]
    fn test_parse_empty_lines() {
        let script = "\n\n\\set x 1\n\n\nSELECT 1;\n\n";
        let commands = parse_script(script).unwrap();
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_parse_error_unknown_command() {
        let script = "\\foo";
        let result = parse_script(script);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_set_no_args() {
        let script = "\\set";
        let result = parse_script(script);
        assert!(result.is_err());
    }
}
