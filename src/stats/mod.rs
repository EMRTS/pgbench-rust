//! Statistics collection and reporting
//!
//! This module handles collecting performance metrics and generating reports.

pub mod collector;
pub mod reporter;

pub use collector::StatsCollector;
pub use reporter::{print_latency_details, print_progress, print_results, print_summary};
