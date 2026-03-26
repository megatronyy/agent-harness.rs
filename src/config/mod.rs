//! Configuration system module
//!
//! This module will contain the configuration structures and loader.

pub mod app;
pub mod loader;
pub mod model;

// Re-export common types
pub use app::AppConfig;
pub use loader::load_config;
pub use model::ModelConfig;

// TODO: Add more configuration modules
// pub mod agents;
// pub mod sandbox;
// pub mod memory;
