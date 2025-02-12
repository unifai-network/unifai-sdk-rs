//! This module provides essential tools for integrating Unifai into your agent.
//!
//! # Example
//!
//! See examples/openai_agent.rs
//!
//! ```no_run
#![doc = include_str!("../../examples/openai_agent.rs")]
//! ```

mod call_tool;
pub use call_tool::*;

mod search_tools;
pub use search_tools::*;

/// Returns two essential tools to integrate Unifai with your agent.
pub fn get_tools(api_key: &str) -> (SearchTools, CallTool) {
    (SearchTools::new(api_key), CallTool::new(api_key))
}
