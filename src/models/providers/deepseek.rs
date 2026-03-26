//! DeepSeek model provider
//!
//! This module implements the ChatModel trait for DeepSeek's API.
//! DeepSeek uses an OpenAI-compatible API format.

use crate::{
    config::ModelConfig,
    error::{HarnessError, ModelError, Result},
    messages::Content,
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

/// DeepSeek API request (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    stop: Option<StopSeq>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum StopSeq {
    Single(String),
    Multiple(Vec<String>),
}

/// DeepSeek message (OpenAI-compatible)
#[derive(Debug, Serialize, Deserialize)]
struct DeepSeekMessage {
    role: String,
    content: String,
}

/// DeepSeek API response (OpenAI-compatible)
#[derive(Debug, Deserialize)]
struct DeepSeekResponse {
    id: String,
    choices: Vec<Choice>,
    usage: Option<DeepSeekUsage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<DeepSeekToolCall>>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct DeepSeekUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// DeepSeek streaming delta
#[derive(Debug, Deserialize)]
struct StreamDelta {
    role: Option<String>,
    content: Option<String>,
}

/// DeepSeek streaming choice
#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

/// DeepSeek model
pub struct DeepSeekModel {
    name: String,
    model_id: String,
    api_key: String,
    base_url: String,
    capabilities: ModelCapabilities,
    client: Client,
}

impl DeepSeekModel {
    /// Create a new DeepSeek model from configuration
    pub fn new(config: &ModelConfig) -> Result<Self> {
        // Extract API key from config
        let api_key = config
            .config
            .as_ref()
            .and_then(|c| c.get("api_key"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HarnessError::Model(ModelError::InvalidApiKey(
                    "deepseek".to_string(),
                ))
            })?;

        // Resolve environment variable if needed
        let api_key = if api_key.starts_with('$') {
            std::env::var(&api_key[1..]).map_err(|_| {
                HarnessError::Model(ModelError::InvalidApiKey(
                    "deepseek".to_string(),
                ))
            })?
        } else {
            api_key.to_string()
        };

        let model_id = config.name.clone();

        let capabilities = ModelCapabilities::new();

        let base_url = config
            .config
            .as_ref()
            .and_then(|c| c.get("base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.deepseek.com")
            .to_string();

        Ok(Self {
            name: config.name.clone(),
            model_id,
            api_key,
            base_url,
            capabilities,
            client: Client::new(),
        })
    }

    /// Convert Message to DeepSeek format
    fn convert_message(msg: &Message) -> DeepSeekMessage {
        let role = match msg.role {
            MessageRole::Human => "user",
            MessageRole::Ai => "assistant",
            MessageRole::Tool => "tool",
            MessageRole::System => "system",
        };

        let content = msg.content.to_string();

        DeepSeekMessage {
            role: role.to_string(),
            content,
        }
    }

    /// Convert DeepSeek response to ModelResponse
    fn convert_response(resp: DeepSeekResponse) -> ModelResponse {
        let choice = resp.choices.first();

        let content = choice
            .and_then(|c| c.message.content.as_ref())
            .cloned()
            .unwrap_or_default();

        let tool_calls = choice
            .and_then(|c| c.message.tool_calls.as_ref())
            .map(|calls| {
                calls
                    .iter()
                    .map(|c| {
                        let arguments = serde_json::from_str::<serde_json::Value>(
                            &c.function.arguments,
                        )
                        .unwrap_or_else(|_| serde_json::json!({}));
                        ToolCall {
                            id: c.id.clone(),
                            name: c.function.name.clone(),
                            arguments,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = resp.usage.map(|u| TokenUsage::new(u.prompt_tokens, u.completion_tokens));

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
            finish_reason: choice.and_then(|c| c.finish_reason.clone()),
        }
    }
}

#[async_trait]
impl ChatModel for DeepSeekModel {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &ModelCapabilities {
        &self.capabilities
    }

    async fn invoke(&self, request: ModelRequest) -> Result<ModelResponse> {
        let messages: Vec<DeepSeekMessage> =
            request.messages.iter().map(Self::convert_message).collect();

        let stop = if request.stop.is_empty() {
            None
        } else if request.stop.len() == 1 {
            Some(StopSeq::Single(request.stop[0].clone()))
        } else {
            Some(StopSeq::Multiple(request.stop))
        };

        let api_request = DeepSeekRequest {
            model: self.model_id.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&api_request)
            .send()
            .await
            .map_err(|e| {
                HarnessError::Model(ModelError::InvocationFailed(format!(
                    "DeepSeek API request failed: {}",
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
                "DeepSeek API error ({}): {}",
                status, body
            ))));
        }

        let api_response: DeepSeekResponse = serde_json::from_str(&body).map_err(|e| {
            HarnessError::Model(ModelError::InvalidResponse(format!(
                "Failed to parse DeepSeek response: {}",
                e
            )))
        })?;

        Ok(Self::convert_response(api_response))
    }

    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        let messages: Vec<DeepSeekMessage> =
            request.messages.iter().map(Self::convert_message).collect();

        let stop = if request.stop.is_empty() {
            None
        } else if request.stop.len() == 1 {
            Some(StopSeq::Single(request.stop[0].clone()))
        } else {
            Some(StopSeq::Multiple(request.stop))
        };

        let api_request = DeepSeekRequest {
            model: self.model_id.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop,
        };

        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();

        let stream = async_stream::stream! {
            let response = match client
                .post(format!("{}/chat/completions", base_url))
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .json(&api_request)
                .query(&[("stream", "true")])
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    yield Err(HarnessError::Model(ModelError::InvocationFailed(
                        format!("DeepSeek API request failed: {}", e)
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
                    format!("DeepSeek API error ({}): {}", status, body)
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

                    // Parse streaming response
                    #[derive(Debug, Deserialize)]
                    struct StreamResponse {
                        choices: Vec<StreamChoice>,
                    }

                    match serde_json::from_str::<StreamResponse>(json_str) {
                        Ok(resp) => {
                            if let Some(choice) = resp.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    yield Ok(StreamEvent::TokenDelta {
                                        delta: content.clone(),
                                        index: 0,
                                    });
                                }
                                if choice.finish_reason.is_some() {
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
    fn test_deepseek_model_creation_fails_without_api_key() {
        let config = ModelConfig {
            name: "deepseek-chat".to_string(),
            provider: "langchain_deepseek:ChatDeepSeek".to_string(),
            supports_thinking: false,
            supports_vision: false,
            config: None,
        };

        let result = DeepSeekModel::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_deepseek_model_creation_with_api_key() {
        let config = ModelConfig {
            name: "deepseek-chat".to_string(),
            provider: "langchain_deepseek:ChatDeepSeek".to_string(),
            supports_thinking: false,
            supports_vision: false,
            config: Some(serde_yaml::Mapping::from_iter(vec![
                (serde_yaml::Value::String("api_key".to_string()), serde_yaml::Value::String("sk-test".to_string())),
            ]).into()),
        };

        let result = DeepSeekModel::new(&config);
        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.name, "deepseek-chat");
    }

    #[test]
    fn test_convert_message() {
        let msg = Message {
            role: MessageRole::Human,
            content: Content::text("Hello, DeepSeek!"),
            tool_calls: vec![],
            tool_call_id: None,
            metadata: None,
            usage: None,
        };

        let ds_msg = DeepSeekModel::convert_message(&msg);
        assert_eq!(ds_msg.role, "user");
        assert_eq!(ds_msg.content, "Hello, DeepSeek!");
    }
}
