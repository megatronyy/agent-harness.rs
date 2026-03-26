//! Guardrails system
//!
//! This module provides the guardrails system for validating and
//! controlling agent tool calls.

pub mod allowlist;
pub mod provider;

// Re-export common types
pub use allowlist::AllowlistProvider;
pub use provider::{GuardrailCheck, GuardrailProvider, GuardrailResult};
