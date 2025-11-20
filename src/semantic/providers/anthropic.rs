//! Anthropic API provider implementation

use super::LlmProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;

/// Anthropic provider for Claude models
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "claude-3-5-haiku-20241022".to_string()),
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, prompt: &str, _json_mode: bool) -> Result<String> {
        // Anthropic doesn't have a JSON mode - it returns plain text by default
        // The json_mode parameter is ignored
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "max_tokens": 4000,
                "temperature": 0.1,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ]
            }))
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Anthropic API error ({}): {}", status, error_text);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Anthropic response as JSON")?;

        // Extract content from response
        let content = data["content"][0]["text"]
            .as_str()
            .context("No content in Anthropic response")?;

        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        "claude-3-5-haiku-20241022"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_default_model() {
        let provider = AnthropicProvider::new("test-key".to_string(), None).unwrap();
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.model, "claude-3-5-haiku-20241022");
    }

    #[test]
    fn test_new_with_custom_model() {
        let provider = AnthropicProvider::new(
            "test-key".to_string(),
            Some("claude-3-5-sonnet-20241022".to_string())
        ).unwrap();
        assert_eq!(provider.model, "claude-3-5-sonnet-20241022");
    }
}
