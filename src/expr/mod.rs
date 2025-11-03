//! Expression parser and evaluator
//!
//! This module implements the pgbench expression language, which supports:
//! - Variables (:varname)
//! - Arithmetic operators (+, -, *, /, %)
//! - Comparison operators (=, <, >, <=, >=, !=)
//! - Logical operators (AND, OR, NOT)
//! - Bitwise operators (&, |, #, ~, <<, >>)
//! - Built-in functions (random, sqrt, abs, etc.)
//! - CASE expressions

pub mod parser;
pub mod eval;

pub use parser::parse_expression;
pub use eval::evaluate_expression;

// TODO: Choose parser implementation (pest, nom, or lalrpop)
// TODO: Implement lexer
// TODO: Implement parser
// TODO: Implement evaluator
// TODO: Implement all built-in functions
