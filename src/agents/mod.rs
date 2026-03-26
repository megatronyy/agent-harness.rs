//! Agent system module
//!
//! This module will contain the Agent builder, executor, and related functionality.

pub mod executor;
pub mod middleware;
pub mod state;

// Re-export common types
pub use executor::{AgentBuilder, AgentExecutor};
pub use middleware::{Middleware, MiddlewareChain, MiddlewareContext, MiddlewareHook};
pub use state::{AgentState, ImageData, ThreadState};

// TODO: Add more submodules as we implement them
// pub mod lead_agent;
