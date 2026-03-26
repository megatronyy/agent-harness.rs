//! Tool system
//!
//! This module provides the tool system for agent tools,
//! including the Tool trait, tool registry, and executor.

pub mod base;
pub mod builtin;
pub mod executor;
pub mod registry;

// Re-export common types
pub use base::{Tool, ToolArgs, ToolContext, ToolOutput, ToolResult};
pub use executor::ToolExecutor;
pub use registry::ToolRegistry;
