//! Title middleware
//!
//! This middleware automatically generates thread titles based on the conversation.

use crate::{
    agents::{
        middleware::base::{Middleware, MiddlewareContext, MiddlewareHook},
        state::ThreadState,
    },
    error::{HarnessError, MiddlewareError},
    models::base::{Message, MessageRole, ModelRequest},
    Result,
};
use async_trait::async_trait;

/// Title generation middleware
///
/// Automatically generates a thread title after the first complete exchange.
pub struct TitleMiddleware {
    name: String,
    hooks: Vec<MiddlewareHook>,
    max_words: usize,
    max_chars: usize,
}

impl TitleMiddleware {
    /// Create a new title middleware
    pub fn new() -> Self {
        Self {
            name: "title".to_string(),
            hooks: vec![MiddlewareHook::AfterModel],
            max_words: 10,
            max_chars: 50,
        }
    }

    /// Set the maximum number of words in the title
    pub fn with_max_words(mut self, max_words: usize) -> Self {
        self.max_words = max_words;
        self
    }

    /// Set the maximum number of characters in the title
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    /// Generate a title from the conversation
    fn generate_title(&self, state: &ThreadState) -> Result<String> {
        // Find the first human message
        let first_human = state
            .messages
            .iter()
            .find(|m| m.role == MessageRole::Human);

        let content = match first_human {
            Some(msg) => msg.content.to_string(),
            None => return Ok("New Thread".to_string()),
        };

        // Clean and truncate the content
        let title = self.clean_title(content);

        Ok(title)
    }

    /// Clean and format the title
    fn clean_title(&self, content: String) -> String {
        // Remove special characters and normalize whitespace
        let cleaned = content
            .chars()
            .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
            .collect::<String>();

        // Split into words, take first N, and capitalize each word
        let words: Vec<String> = cleaned
            .split_whitespace()
            .take(self.max_words)
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().to_string() + chars.as_str()
                    }
                }
            })
            .collect();

        let mut title = words.join(" ");

        // Truncate to max characters if needed
        if title.len() > self.max_chars {
            title = truncate_at_word_boundary(&title, self.max_chars);
        }

        title
    }
}

/// Truncate string at word boundary
fn truncate_at_word_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }

    // Find the last space before max_len
    let truncated = &s[..max_len];
    if let Some(last_space) = truncated.rfind(' ') {
        truncated[..last_space].to_string()
    } else {
        truncated.to_string()
    }
}

impl Default for TitleMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for TitleMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    fn hooks(&self) -> &[MiddlewareHook] {
        &self.hooks
    }

    async fn execute(&self, context: &mut MiddlewareContext) -> Result<()> {
        // Only generate title if there's at least one exchange
        // (human message followed by AI response)
        let messages = &context.state.messages;

        let has_human = messages.iter().any(|m| m.role == MessageRole::Human);
        let has_ai = messages.iter().any(|m| m.role == MessageRole::Ai);

        if has_human && has_ai {
            let title = self.generate_title(&context.state)?;

            // Update state with title
            context.state.title = Some(title.clone());

            // Update metadata
            context.metadata["generated_title"] = serde_json::Value::String(title);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_middleware_name() {
        let middleware = TitleMiddleware::new();
        assert_eq!(middleware.name(), "title");
    }

    #[test]
    fn test_title_middleware_hooks() {
        let middleware = TitleMiddleware::new();
        assert_eq!(middleware.hooks(), &[MiddlewareHook::AfterModel]);
    }

    #[test]
    fn test_title_middleware_with_max_words() {
        let middleware = TitleMiddleware::new().with_max_words(5);
        assert_eq!(middleware.max_words, 5);
    }

    #[test]
    fn test_clean_title() {
        let middleware = TitleMiddleware::new();

        let title = middleware.clean_title("Hello, World! How are you today?".to_string());
        assert!(title.len() <= 50);
        assert_eq!(title, "Hello World How Are You Today");
    }

    #[test]
    fn test_clean_title_with_special_chars() {
        let middleware = TitleMiddleware::new();

        let title = middleware.clean_title("Test@#$%Title!@#$%".to_string());
        assert_eq!(title, "Test Title");
    }

    #[test]
    fn test_truncate_at_word_boundary() {
        let s = "Hello World Test String";
        let result = truncate_at_word_boundary(s, 15);
        assert_eq!(result, "Hello World");
    }
}
