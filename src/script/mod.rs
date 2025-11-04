//! Transaction script parsing and execution
//!
//! This module handles parsing and executing pgbench transaction scripts,
//! including SQL statements and meta-commands.

pub mod builtin;
pub mod parser;
pub mod executor;

pub use builtin::BuiltinScript;
pub use parser::parse_script;
pub use executor::execute_script;

// TODO: Implement script parser
// TODO: Implement meta-command support (\set, \sleep, etc.)
// TODO: Implement conditional execution (\if, \elif, \else, \endif)
// TODO: Implement pipeline mode (\startpipeline, \endpipeline)
