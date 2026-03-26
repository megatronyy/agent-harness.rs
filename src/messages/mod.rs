//! Message type system module
//!
//! This module contains all message types used in agent communication.

pub mod content;

// Re-export message types
pub use content::{Content, ContentBlock};

// TODO: Add more message types
// pub mod base;
// pub mod human;
// pub mod ai;
// pub mod tool;
