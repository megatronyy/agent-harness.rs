//! Content types for messages
//!
//! This module defines the `Content` enum which can represent text,
//! images, or mixed content in messages.

use serde::{Deserialize, Serialize};

/// Content that can be included in a message
///
/// This enum represents different types of content that can be sent
/// to LLMs, including text, images, and structured data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Content {
    /// Plain text content
    Text(String),

    /// Image content with URL or base64 data
    Image {
        /// MIME type (e.g., "image/png", "image/jpeg")
        mime_type: String,
        /// Image data (URL or base64)
        data: String,
    },

    /// Mixed/structured content
    ///
    /// Used for complex messages with multiple parts, such as
    /// text plus images, or structured data.
    Mixed(Vec<ContentBlock>),
}

/// A block of content in a mixed message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentBlock {
    /// Type of content block
    #[serde(rename = "type")]
    pub block_type: ContentBlockType,

    /// Content data
    pub content: serde_json::Value,
}

/// Type of content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockType {
    /// Text block
    Text,
    /// Image URL block
    ImageUrl,
}

impl Content {
    /// Create new text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Create new image content from URL
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            mime_type: "image/jpeg".to_string(),
            data: url.into(),
        }
    }

    /// Create new image content from base64 data
    pub fn image_base64(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            mime_type: mime_type.into(),
            data: data.into(),
        }
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(s) => s.is_empty(),
            Self::Image { .. } => false,
            Self::Mixed(blocks) => blocks.is_empty(),
        }
    }

    /// Get text representation of content
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Convert content to string (best effort)
    pub fn to_string(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Image { mime_type, .. } => format!("[Image: {}]", mime_type),
            Self::Mixed(blocks) => {
                blocks
                    .iter()
                    .map(|b| match &b.content {
                        serde_json::Value::String(s) => s.clone(),
                        _ => "[Complex Content]".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl<T: Into<String>> From<Vec<T>> for Content {
    fn from(texts: Vec<T>) -> Self {
        let blocks = texts
            .into_iter()
            .map(|text| ContentBlock {
                block_type: ContentBlockType::Text,
                content: serde_json::json!(text.into()),
            })
            .collect();
        Self::Mixed(blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_text() {
        let content = Content::text("Hello, world!");
        assert!(content.as_text().is_some());
        assert_eq!(content.as_text().unwrap(), "Hello, world!");
        assert!(!content.is_empty());
    }

    #[test]
    fn test_content_empty() {
        let content = Content::text("");
        assert!(content.is_empty());
    }

    #[test]
    fn test_content_image_url() {
        let content = Content::image_url("https://example.com/image.png");
        assert!(content.as_text().is_none());
        assert!(!content.is_empty());
    }

    #[test]
    fn test_content_image_base64() {
        let content = Content::image_base64("image/png", "iVBORw0KGgo...");
        assert!(!content.is_empty());
    }

    #[test]
    fn test_content_from_string() {
        let content: Content = "Hello".to_string().into();
        assert_eq!(content.as_text().unwrap(), "Hello");
    }

    #[test]
    fn test_content_from_str() {
        let content: Content = "Hello".into();
        assert_eq!(content.as_text().unwrap(), "Hello");
    }

    #[test]
    fn test_content_to_string() {
        let text_content = Content::text("Hello");
        assert_eq!(text_content.to_string(), "Hello");

        let image_content = Content::image_url("https://example.com/img.png");
        assert!(image_content.to_string().contains("Image"));
    }

    #[test]
    fn test_content_mixed() {
        let blocks = vec![
            ContentBlock {
                block_type: ContentBlockType::Text,
                content: serde_json::json!("Hello"),
            },
            ContentBlock {
                block_type: ContentBlockType::Text,
                content: serde_json::json!("World"),
            },
        ];
        let content = Content::Mixed(blocks);
        assert!(!content.is_empty());
    }
}
