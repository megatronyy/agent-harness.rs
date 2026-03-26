//! Subagent system
//!
//! This module provides the subagent delegation system for
//! distributing tasks to specialized agents.

pub mod executor;
pub mod registry;

// Re-export common types
pub use executor::{SubagentExecutor, SubagentTask, TaskStatus};
pub use registry::{AgentDefinition, AgentRegistry};
