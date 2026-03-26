//! Runtime system module
//!
//! This module contains runtime context and configuration.

pub mod context;

// Re-export common types
pub use context::{RuntimeContext, RuntimeMetadata};

// TODO: Add more runtime modules
// pub mod config;
