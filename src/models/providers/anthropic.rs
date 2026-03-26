//! Anthropic Claude model provider
//!
//! This module implements the ChatModel trait for Anthropic's Claude API,
//! supporting Claude Opus, Sonnet, and Haiku models with extended thinking
//! and vision capabilities.

use crate::{
    config::ModelConfig,
    error::{HarnessError, ModelError, Result},
    messages::{content::ContentBlockType, Content},
    models::base::{
        ChatModel, Message, MessageRole, ModelCapabilities, ModelRequest, ModelResponse,
        StreamEvent, TokenUsage, ToolCall,
    },
};
use async_trait::async_trait;
use futures::stream::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Anthropic API request
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    stop_sequences: Option<Vec<String>>,
    system: Option<String>,
    thinking: Option<bool>,
    betas: Option<Vec<String>>,
}

/// Anthropic message
#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

/// Anthropic content block
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContent {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult { tool_use_id: String, content: String },
}

/// Image source for vision
#[derive(Debug, Serialize, Deserialize)]
struct ImageSource {
    #[serde(rename = "type")]
    media_type: String,
    data: String,
}

/// Anthropic API response
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    role: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

/// Anthropic usage info
#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic streaming event
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicStreamEvent {
    MessageStart,
    MessageDelta { delta: Delta, usage: AnthropicUsage },
    MessageStop,
    ContentBlockStart { index: u32 },
    ContentBlockDelta { index: u32, delta: Delta },
    ContentBlockStop { index: u32 },
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    text: String,
    stop_reason: Option<String>,
}

/// Anthropic Claude model
pub struct AnthropicModel {
    name: String,
    model_id: String,
    api_key: String,
    base_url: String,
    capabilities: ModelCapabilities,
    client: Client,
    thinking_enabled: bool,
}

impl AnthropicModel {
    /// Create a new Anthropic model from configuration
    pub fn new(config: &ModelConfig) -> Result<Self> {
        // Extract API key from config
        let api_key = config
            .config
            .as_ref()
            .and_then(|c| c.get("api_key"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HarnessError::Model(ModelError::InvalidApiKey(
                    "anthropic".to_string(),
                ))
            })?;

        // Resolve environment variable if needed
        let api_key = if api_key.starts_with('$') {
            std::env::var(&api_key[1..]).map_err(|_| {
                HarnessError::Model(ModelError::InvalidApiKey(
                    "anthropic".to_string(),
                ))
            })?
        } else {
            api_key.to_string()
        };

        let model_id = config.name.clone();

        // Determine capabilities based on model name and config
        let mut capabilities = ModelCapabilities::new();
        if config.supports_thinking {
            capabilities = capabilities.with_thinking();
        }
        if config.supports_vision {
            capabilities = capabilities.with_vision();
        }

        let base_url = config
            .config
            .as_ref()
            .and_then(|c| c.get("base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.anthropic.com")
            .to_string();

        let thinking_enabled = config
            .config
            .as_ref()
            .and_then(|c| c.get("thinking_enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Self {
            name: config.name.clone(),
            model_id,
            api_key,
            base_url,
            capabilities,
            client: Client::new(),
            thinking_enabled,
        })
    }

    /// Convert Message to Anthropic format
    fn convert_messages(messages: &[Message]) -> Vec<AnthropicMessage> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::Human => "user",
                    MessageRole::Ai => "assistant",
                    MessageRole::Tool => "user",
                    MessageRole::System => "user",
                };

                let content = Self::convert_content(msg);

                AnthropicMessage {
                    role: role.to_string(),
                    content,
                }
            })
            .collect()
    }

    /// Convert message content to Anthropic format
    fn convert_content(msg: &Message) -> Vec<AnthropicContent> {
        match &msg.content {
            Content::Text(text) => {
                vec![AnthropicContent::Text {
                    text: text.clone(),
                }]
            }
            Content::Image { mime_type, data } => {
                vec![AnthropicContent::Image {
                    source: ImageSource {
                        media_type: mime_type.clone(),
                        data: data.clone(),
                    },
                }]
            }
            Content::Mixed(blocks) => blocks
                .iter()
                .map(|block| {
                    if block.block_type == ContentBlockType::Text {
                        let text = block.content.as_str().unwrap_or("").to_string();
                        AnthropicContent::Text { text }
                    } else if block.block_type == ContentBlockType::ImageUrl {
                        // For image blocks, we expect the content to have url or data
                        if let Some(url) = block.content.get("url").and_then(|v| v.as_str()) {
                            AnthropicContent::Image {
                                source: ImageSource {
                                    media_type: "image/jpeg".to_string(),
                                    data: url.to_string(),
                                },
                            }
                        } else if let Some(data) = block.content.get("data").and_then(|v| v.as_str()) {
                            let mime_type = block.content.get("mime_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("image/png");
                            AnthropicContent::Image {
                                source: ImageSource {
                                    media_type: mime_type.to_string(),
                                    data: data.to_string(),
                                },
                            }
                        } else {
                            AnthropicContent::Text {
                                text: "[Unsupported image content]".to_string(),
                            }
                        }
                    } else {
                        AnthropicContent::Text {
                            text: "[Unknown content type]".to_string(),
                        }
                    }
                })
                .collect(),
        }
    }

    /// Convert Anthropic response to ModelResponse
    fn convert_response(resp: AnthropicResponse) -> ModelResponse {
        let content = resp
            .content
            .iter()
            .filter_map(|c| match c {
                AnthropicContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let tool_calls = resp
            .content
            .iter()
            .filter_map(|c| match c {
                AnthropicContent::ToolUse { id, name, input } => Some(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                }),
                _ => None,
            })
            .collect();

        let usage = resp.usage.map(|u| TokenUsage::new(u.input_tokens, u.output_tokens));

        ModelResponse {
            message: Message {
                role: MessageRole::Ai,
                content: Content::text(content),
                tool_calls,
                tool_call_id: None,
                metadata: None,
                usage: usage.clone(),
            },
            usage,
            finish_reason: resp.stop_reason,
        }
    }
}

#[async_trait]
impl ChatModel for AnthropicModel {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &ModelCapabilities {
        &self.capabilities
    }

    async fn invoke(&self, request: ModelRequest) -> Result<ModelResponse> {
        // Extract system message
        let system_message = request
            .messages
            .iter()
            .find(|m| m.role == MessageRole::System)
            .and_then(|m| match &m.content {
                Content::Text(text) => Some(text.clone()),
                _ => None,
            });

        // Filter out system messages from the request
        let messages: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .cloned()
            .collect();

        let api_request = AnthropicRequest {
            model: self.model_id.clone(),
            messages: Self::convert_messages(&messages),
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k,
            stop_sequences: if request.stop.is_empty() {
                None
            } else {
                Some(request.stop)
            },
            system: system_message,
            thinking: if self.thinking_enabled { Some(true) } else { None },
            betas: if self.thinking_enabled {
                Some(vec!["output-128k-2025-02-19".to_string()])
            } else {
                None
            },
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| {
                HarnessError::Model(ModelError::InvocationFailed(format!(
                    "Anthropic API request failed: {}",
                    e
                )))
            })?;

        let status = response.status();
        let body = response.text().await.map_err(|e| {
            HarnessError::Model(ModelError::InvocationFailed(format!(
                "Failed to read response body: {}",
                e
            )))
        })?;

        if !status.is_success() {
            return Err(HarnessError::Model(ModelError::InvocationFailed(format!(
                "Anthropic API error ({}): {}",
                status, body
            ))));
        }

        let api_response: AnthropicResponse =
            serde_json::from_str(&body).map_err(|e| {
                HarnessError::Model(ModelError::InvalidResponse(format!(
                    "Failed to parse Anthropic response: {}",
                    e
                )))
            })?;

        Ok(Self::convert_response(api_response))
    }

    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        // Extract system message
        let system_message = request
            .messages
            .iter()
            .find(|m| m.role == MessageRole::System)
            .and_then(|m| match &m.content {
                Content::Text(text) => Some(text.clone()),
                _ => None,
            });

        // Filter out system messages
        let messages: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .cloned()
            .collect();

        let api_request = AnthropicRequest {
            model: self.model_id.clone(),
            messages: Self::convert_messages(&messages),
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k,
            stop_sequences: if request.stop.is_empty() {
                None
            } else {
                Some(request.stop)
            },
            system: system_message,
            thinking: if self.thinking_enabled { Some(true) } else { None },
            betas: if self.thinking_enabled {
                Some(vec!["output-128k-2025-02-19".to_string()])
            } else {
                None
            },
        };

        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();

        let stream = async_stream::stream! {
            let response = match client
                .post(format!("{}/v1/messages", base_url))
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&api_request)
                .header("accept", "text/event-stream")
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    yield Err(HarnessError::Model(ModelError::InvocationFailed(
                        format!("Anthropic API request failed: {}", e)
                    )));
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let body = match response.text().await {
                    Ok(b) => b,
                    Err(_) => "<unable to read body>".to_string(),
                };
                yield Err(HarnessError::Model(ModelError::InvocationFailed(
                    format!("Anthropic API error ({}): {}", status, body)
                )));
                return;
            }

            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();

            use futures::StreamExt;
            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(HarnessError::Model(ModelError::StreamError(
                            format!("Stream error: {}", e)
                        )));
                        return;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    let line = line.trim();
                    if line.is_empty() || !line.starts_with("data:") {
                        continue;
                    }

                    let json_str = &line[5..].trim();
                    if *json_str == "[DONE]" {
                        yield Ok(StreamEvent::End);
                        return;
                    }

                    match serde_json::from_str::<AnthropicStreamEvent>(json_str) {
                        Ok(event) => {
                            match event {
                                AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                                    if !delta.text.is_empty() {
                                        yield Ok(StreamEvent::TokenDelta {
                                            delta: delta.text,
                                            index: 0,
                                        });
                                    }
                                    if delta.stop_reason.is_some() {
                                        yield Ok(StreamEvent::MessageComplete {
                                            message: Message {
                                                role: MessageRole::Ai,
                                                content: Content::text(""),
                                                tool_calls: vec![],
                                                tool_call_id: None,
                                                metadata: None,
                                                usage: None,
                                            },
                                        });
                                    }
                                }
                                AnthropicStreamEvent::MessageDelta { usage, .. } => {
                                    yield Ok(StreamEvent::MessageComplete {
                                        message: Message {
                                            role: MessageRole::Ai,
                                            content: Content::text(""),
                                            tool_calls: vec![],
                                            tool_call_id: None,
                                            metadata: None,
                                            usage: Some(TokenUsage::new(usage.input_tokens, usage.output_tokens)),
                                        },
                                    });
                                }
                                AnthropicStreamEvent::MessageStop => {
                                    yield Ok(StreamEvent::End);
                                    return;
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            yield Err(HarnessError::Model(ModelError::InvalidResponse(
                                format!("Failed to parse stream event: {}", e)
                            )));
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_model_creation_fails_without_api_key() {
        let config = ModelConfig {
            name: "claude-3-5-sonnet-20241022".to_string(),
            provider: "langchain_anthropic:ChatAnthropic".to_string(),
            supports_thinking: true,
            supports_vision: true,
            config: None,
        };

        let result = AnthropicModel::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_anthropic_model_creation_with_api_key() {
        let config = ModelConfig {
            name: "claude-3-5-sonnet-20241022".to_string(),
            provider: "langchain_anthropic:ChatAnthropic".to_string(),
            supports_thinking: true,
            supports_vision: true,
            config: Some(serde_yaml::Mapping::from_iter(vec![
                (serde_yaml::Value::String("api_key".to_string()), serde_yaml::Value::String("sk-test".to_string())),
            ]).into()),
        };

        let result = AnthropicModel::new(&config);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "claude-3-5-sonnet-20241022");
    }
}
