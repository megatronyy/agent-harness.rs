//! Middleware system
//!
//! This module provides the middleware architecture for the agent-harness system.
//! Middlewares allow intercepting and modifying agent execution at various points.

pub mod base;
pub mod chain;
pub mod middlewares;

// Re-export common types
pub use base::{Middleware, MiddlewareContext, MiddlewareHook};
pub use chain::MiddlewareChain;
