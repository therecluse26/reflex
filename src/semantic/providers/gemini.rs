//! Google Gemini API provider implementation

use super::LlmProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;

/// Gemini provider for Google models
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "gemini-2.5-flash".to_string()),
        })
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, prompt: &str, json_mode: bool) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        // Use JSON MIME type if json_mode is true, otherwise plain text
        let mime_type = if json_mode {
            "application/json"
        } else {
            "text/plain"
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&json!({
                "contents": [
                    {
                        "parts": [
                            {
                                "text": prompt
                            }
                        ]
                    }
                ],
                "generationConfig": {
                    "temperature": 0.1,
                    "maxOutputTokens": 4000,
                    "responseMimeType": mime_type
                }
            }))
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Gemini API error ({}): {}", status, error_text);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Gemini response as JSON")?;

        // Extract content from response
        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .context("No content in Gemini response")?;

        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "gemini"
    }

    fn default_model(&self) -> &str {
        "gemini-2.5-flash"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_default_model() {
        let provider = GeminiProvider::new("test-key".to_string(), None).unwrap();
        assert_eq!(provider.name(), "gemini");
        assert_eq!(provider.model, "gemini-2.5-flash");
    }

    #[test]
    fn test_new_with_custom_model() {
        let provider = GeminiProvider::new(
            "test-key".to_string(),
            Some("gemini-2.5-pro".to_string())
        ).unwrap();
        assert_eq!(provider.model, "gemini-2.5-pro");
    }
}
