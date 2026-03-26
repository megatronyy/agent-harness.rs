//! MCP (Model Context Protocol) integration
//!
//! This module provides MCP client and tool adapter for integrating
//! external MCP servers with the agent system.

pub mod adapter;
pub mod client;

// Re-export common types
pub use adapter::McpToolAdapter;
pub use client::{McpClient, McpClientConfig};
