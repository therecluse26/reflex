//! LLM provider implementations

pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod groq;

use anyhow::Result;
use async_trait::async_trait;

/// Trait for LLM providers that generate structured query responses
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a prompt and get JSON response
    ///
    /// The response should be valid JSON matching the QueryResponse schema.
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// Get provider name (for logging and error messages)
    fn name(&self) -> &str;

    /// Get default model identifier
    fn default_model(&self) -> &str;
}

/// Create a provider instance from name and API key
pub fn create_provider(
    provider_name: &str,
    api_key: String,
    model: Option<String>,
) -> Result<Box<dyn LlmProvider>> {
    match provider_name.to_lowercase().as_str() {
        "openai" => Ok(Box::new(openai::OpenAiProvider::new(api_key, model)?)),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(api_key, model)?)),
        "gemini" => Ok(Box::new(gemini::GeminiProvider::new(api_key, model)?)),
        "groq" => Ok(Box::new(groq::GroqProvider::new(api_key, model)?)),
        _ => anyhow::bail!(
            "Unknown provider: {}. Supported: openai, anthropic, gemini, groq",
            provider_name
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider_openai() {
        let provider = create_provider("openai", "test-key".to_string(), None);
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().name(), "openai");
    }

    #[test]
    fn test_create_provider_case_insensitive() {
        let provider = create_provider("OpenAI", "test-key".to_string(), None);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_provider_unknown() {
        let provider = create_provider("unknown", "test-key".to_string(), None);
        assert!(provider.is_err());
        assert!(provider.unwrap_err().to_string().contains("Unknown provider"));
    }
}
