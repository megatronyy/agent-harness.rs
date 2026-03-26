//! Application configuration

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main application configuration
///
/// This structure holds all configuration values for the agent-harness library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// List of configured models
    pub models: Vec<ModelConfig>,

    /// Tool configurations
    pub tools: Vec<ToolConfig>,

    /// Sandbox configuration
    pub sandbox: SandboxConfig,

    /// Skills configuration
    pub skills: SkillsConfig,

    /// Memory configuration
    pub memory: MemoryConfig,

    /// Configuration version
    pub config_version: Option<u32>,
}

impl AppConfig {
    /// Load configuration from a YAML file
    pub fn from_yaml<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Get a model configuration by name
    pub fn get_model_config(&self, name: &str) -> Option<&ModelConfig> {
        self.models.iter().find(|m| m.name == name)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            tools: Vec::new(),
            sandbox: SandboxConfig::default(),
            skills: SkillsConfig::default(),
            memory: MemoryConfig::default(),
            config_version: Some(1),
        }
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name/identifier
    pub name: String,

    /// Provider class path (e.g., "langchain_anthropic:ChatAnthropic")
    pub provider: String,

    /// Whether the model supports thinking mode
    pub supports_thinking: bool,

    /// Whether the model supports vision
    pub supports_vision: bool,

    /// Additional provider-specific configuration
    pub config: Option<serde_yaml::Value>,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Tool name
    pub name: String,

    /// Provider class path
    pub provider: String,

    /// Tool group
    pub group: Option<String>,

    /// Additional configuration
    pub config: Option<serde_yaml::Value>,
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Sandbox provider to use
    pub provider: String,

    /// Base path for workspace
    pub workspace_path: Option<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            provider: "local".to_string(),
            workspace_path: None,
        }
    }
}

/// Skills configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Path to skills directory
    pub path: Option<String>,

    /// Container path (inside sandbox)
    pub container_path: Option<String>,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            path: Some("skills".to_string()),
            container_path: Some("/mnt/skills".to_string()),
        }
    }
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Enable memory system
    pub enabled: bool,

    /// Enable memory injection
    pub injection_enabled: bool,

    /// Storage path
    pub storage_path: Option<String>,

    /// Debounce seconds
    pub debounce_seconds: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            injection_enabled: false,
            storage_path: None,
            debounce_seconds: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert!(config.models.is_empty());
        assert!(config.tools.is_empty());
        assert_eq!(config.sandbox.provider, "local");
        assert!(!config.memory.enabled);
    }

    #[test]
    fn test_model_config() {
        let config = ModelConfig {
            name: "claude-opus-4-6".to_string(),
            provider: "langchain_anthropic:ChatAnthropic".to_string(),
            supports_thinking: true,
            supports_vision: true,
            config: None,
        };
        assert_eq!(config.name, "claude-opus-4-6");
        assert!(config.supports_thinking);
    }
}
