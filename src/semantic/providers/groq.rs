//! Groq API provider implementation

use super::LlmProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;

/// Groq provider (OpenAI-compatible API)
pub struct GroqProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GroqProvider {
    /// Create a new Groq provider
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "llama-3.3-70b-versatile".to_string()),
        })
    }
}

#[async_trait]
impl LlmProvider for GroqProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
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
            .context("Failed to send request to Groq API")?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Groq API error ({}): {}", status, error_text);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Groq response as JSON")?;

        // Extract content from response (OpenAI-compatible format)
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .context("No content in Groq response")?;

        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "groq"
    }

    fn default_model(&self) -> &str {
        "llama-3.3-70b-versatile"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_default_model() {
        let provider = GroqProvider::new("test-key".to_string(), None).unwrap();
        assert_eq!(provider.name(), "groq");
        assert_eq!(provider.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn test_new_with_custom_model() {
        let provider = GroqProvider::new(
            "test-key".to_string(),
            Some("mixtral-8x7b-32768".to_string())
        ).unwrap();
        assert_eq!(provider.model, "mixtral-8x7b-32768");
    }
}
