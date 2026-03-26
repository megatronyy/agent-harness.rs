//! Skills system
//!
//! This module provides the skills system for loading and managing
//! agent skill definitions.

pub mod loader;

// Re-export common types
pub use loader::{Skill, SkillLoader, SkillMetadata};
