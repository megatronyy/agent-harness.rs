//! Sandbox system
//!
//! This module provides the sandbox execution system for agent tools,
//! including the Sandbox trait and provider implementations.

pub mod base;
pub mod local;
pub mod provider;
pub mod tools;

// Re-export common types
pub use base::{Sandbox, SandboxCommandResult, SandboxFileResult};
pub use local::LocalSandbox;
pub use provider::{SandboxProvider, SandboxProviderHolder};
