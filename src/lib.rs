//! # agent-harness
//!
//! High-performance AI agent harness framework written in Rust.
//!
//! ## Overview
//!
//! This library provides a comprehensive framework for building AI agents with:
//! - Type-safe message system
//! - Pluggable middleware architecture
//! - Multi-provider LLM support
//! - Tool execution with sandbox isolation
//! - Memory management
//! - Subagent delegation
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use agent_harness::{
//!     prelude::*,
//!     config::AppConfig,
//!     runtime::RuntimeContext,
//! };
//!
//! #[tokio::main]
//! async fn main() -> agent_harness::Result<()> {
//!     let config = AppConfig::default();
//!     let ctx = RuntimeContext::with_thread_id("my-thread");
//!
//!     // TODO: Create and run agent in future phases
//!     println!("Agent harness initialized");
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`agents`] - Agent system with middleware chain
//! - [`config`] - Configuration management
//! - [`error`] - Error types
//! - [`messages`] - Message types (Human, AI, Tool)
//! - [`runtime`] - Runtime context and configuration
//! - [`tools`] - Tool system
//! - [`models`] - LLM model abstractions

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod agents;
pub mod config;
pub mod error;
pub mod guardrails;
pub mod memory;
pub mod messages;
pub mod mcp;
pub mod models;
pub mod runtime;
pub mod sandbox;
pub mod skills;
pub mod subagents;
pub mod tools;

/// Prelude module - common imports that are useful to have in scope
pub mod prelude {
    pub use crate::error::{HarnessError, Result};
    pub use crate::messages::Content;
    pub use crate::models::{ChatModel, ModelCapabilities, ModelRequest, ModelResponse};
    pub use crate::runtime::RuntimeContext;
    pub use crate::tools::base::ToolContext;
    pub use crate::tools::{Tool, ToolRegistry};
}

// Re-export commonly used types at the crate root
pub use crate::error::{HarnessError, Result};
pub use crate::messages::Content;
