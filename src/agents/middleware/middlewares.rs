//! Built-in middlewares
//!
//! This module contains the standard middleware implementations for the agent-harness system.

pub mod dangling_tool_call;
pub mod thread_data;
pub mod title;

// Re-export built-in middlewares
pub use dangling_tool_call::DanglingToolCallMiddleware;
pub use thread_data::ThreadDataMiddleware;
pub use title::TitleMiddleware;
