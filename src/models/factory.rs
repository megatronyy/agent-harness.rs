//! Model factory for creating model instances
//!
//! This module provides a factory for instantiating model providers
//! based on configuration.

use crate::{
    config::ModelConfig,
    error::{HarnessError, ModelError, Result},
    models::base::ChatModel,
};
use std::sync::Arc;

/// Model factory for creating model instances
pub struct ModelFactory;

impl ModelFactory {
    /// Create a model from configuration
    pub fn create_model(config: &ModelConfig) -> Result<Arc<dyn ChatModel>> {
        // Match on provider class path
        match config.provider.as_str() {
            "langchain_anthropic:ChatAnthropic" => {
                #[cfg(feature = "anthropic")]
                {
                    Ok(Arc::new(crate::models::providers::anthropic::AnthropicModel::new(config)?))
                }
                #[cfg(not(feature = "anthropic"))]
                {
                    Err(HarnessError::Model(ModelError::NotFound(
                        "Anthropic provider requires 'anthropic' feature. Enable with: --features anthropic".to_string(),
                    )))
                }
            }
            "langchain_openai:ChatOpenAI" => {
                #[cfg(feature = "openai")]
                {
                    Ok(Arc::new(crate::models::providers::openai::OpenAIModel::new(config)?))
                }
                #[cfg(not(feature = "openai"))]
                {
                    Err(HarnessError::Model(ModelError::NotFound(
                        "OpenAI provider requires 'openai' feature. Enable with: --features openai".to_string(),
                    )))
                }
            }
            "langchain_deepseek:ChatDeepSeek" => {
                #[cfg(feature = "deepseek")]
                {
                    Ok(Arc::new(crate::models::providers::deepseek::DeepSeekModel::new(config)?))
                }
                #[cfg(not(feature = "deepseek"))]
                {
                    Err(HarnessError::Model(ModelError::NotFound(
                        "DeepSeek provider requires 'deepseek' feature. Enable with: --features deepseek".to_string(),
                    )))
                }
            }
            _ => {
                Err(HarnessError::Model(ModelError::NotFound(format!(
                    "Unknown provider class path: {}. Supported providers: langchain_anthropic:ChatAnthropic, langchain_openai:ChatOpenAI, langchain_deepseek:ChatDeepSeek",
                    config.provider
                ))))
            }
        }
    }

    /// Create a model with thinking enabled check
    pub fn create_model_with_thinking(
        config: &ModelConfig,
        _thinking_enabled: bool,
    ) -> Result<Arc<dyn ChatModel>> {
        let model = Self::create_model(config)?;
        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_error_on_unknown_provider() {
        let config = ModelConfig {
            name: "test".to_string(),
            provider: "unknown:provider".to_string(),
            supports_thinking: false,
            supports_vision: false,
            config: None,
        };

        let result = ModelFactory::create_model(&config);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(HarnessError::Model(ModelError::NotFound(_)))
        ));
    }

    #[test]
    fn test_factory_error_anthropic_without_feature() {
        // This test will fail if anthropic feature is enabled
        #[cfg(not(feature = "anthropic"))]
        {
            let config = ModelConfig {
                name: "claude-3-5-sonnet".to_string(),
                provider: "langchain_anthropic:ChatAnthropic".to_string(),
                supports_thinking: true,
                supports_vision: true,
                config: Some(serde_yaml::Mapping::from_iter(vec![
                    (serde_yaml::Value::String("api_key".to_string()), serde_yaml::Value::String("sk-test".to_string())),
                ]).into()),
            };

            let result = ModelFactory::create_model(&config);
            assert!(result.is_err());
        }
    }
}
