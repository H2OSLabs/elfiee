//! Anthropic API client for Agent extension

use super::error::LlmError;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[allow(dead_code)]
    id: String,
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// Anthropic API client for Claude models
pub struct AnthropicClient {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    ///
    /// # Arguments
    /// * `api_key` - Anthropic API key
    /// * `model` - Model name (e.g., "claude-sonnet-4-20250514")
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    /// Send a message to the Anthropic API
    ///
    /// # Arguments
    /// * `system_prompt` - System prompt for the model
    /// * `user_message` - User's message
    /// * `max_tokens` - Maximum tokens in response
    ///
    /// # Returns
    /// The model's text response or an error
    pub async fn send_message(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: u32,
    ) -> Result<String, LlmError> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens,
            system: if system_prompt.is_empty() {
                None
            } else {
                Some(system_prompt.to_string())
            },
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        // Handle error status codes
        if response.status() == 429 {
            return Err(LlmError::RateLimit);
        }

        if response.status() == 401 {
            return Err(LlmError::Unauthorized);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::InvalidResponse(error_text));
        }

        // Parse response
        let api_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        // Extract text from response content blocks
        let text = api_response
            .content
            .into_iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }

    /// Send message with automatic retry on rate limit or network errors
    ///
    /// # Arguments
    /// * `system_prompt` - System prompt for the model
    /// * `user_message` - User's message
    /// * `max_tokens` - Maximum tokens in response
    /// * `max_retries` - Maximum number of retry attempts
    ///
    /// # Returns
    /// The model's text response or the last error after all retries
    pub async fn send_message_with_retry(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: u32,
        max_retries: u32,
    ) -> Result<String, LlmError> {
        let mut last_error = LlmError::Network("No attempts made".to_string());

        for attempt in 0..max_retries {
            match self
                .send_message(system_prompt, user_message, max_tokens)
                .await
            {
                Ok(response) => return Ok(response),
                Err(LlmError::RateLimit) => {
                    // Exponential backoff: 1s, 2s, 4s, 8s...
                    let delay = 2u64.pow(attempt) * 1000;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    last_error = LlmError::RateLimit;
                }
                Err(LlmError::Network(msg)) => {
                    // Linear backoff for network errors: 1s, 2s, 3s...
                    let delay = 1000 * (attempt as u64 + 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    last_error = LlmError::Network(msg);
                }
                Err(e) => return Err(e), // Don't retry other errors (Unauthorized, InvalidResponse)
            }
        }

        Err(last_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            "claude-sonnet-4-20250514".to_string(),
        );
        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_request_serialization() {
        let request = AnthropicRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            system: Some("You are a helpful assistant".to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-sonnet-4-20250514"));
        assert!(json.contains("max_tokens"));
        assert!(json.contains("You are a helpful assistant"));
    }

    #[test]
    fn test_request_without_system_prompt() {
        let request = AnthropicRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            system: None,
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        // system field should be omitted when None
        assert!(!json.contains("system"));
    }
}
