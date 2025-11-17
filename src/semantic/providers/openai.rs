//! OpenAI API provider implementation

use super::LlmProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;

/// OpenAI provider for GPT models
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "temperature": 0.1,
                "max_tokens": 500,
                "response_format": {
                    "type": "json_object"
                }
            }))
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("OpenAI API error ({}): {}", status, error_text);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse OpenAI response as JSON")?;

        // Extract content from response
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .context("No content in OpenAI response")?;

        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        "gpt-4o-mini"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_default_model() {
        let provider = OpenAiProvider::new("test-key".to_string(), None).unwrap();
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.model, "gpt-4o-mini");
    }

    #[test]
    fn test_new_with_custom_model() {
        let provider = OpenAiProvider::new("test-key".to_string(), Some("gpt-4o".to_string())).unwrap();
        assert_eq!(provider.model, "gpt-4o");
    }
}
