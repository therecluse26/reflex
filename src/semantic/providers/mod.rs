//! LLM provider implementations

pub mod openai;
pub mod anthropic;
pub mod groq;

use anyhow::Result;
use async_trait::async_trait;

/// Trait for LLM providers that generate structured query responses
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a prompt and get response
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to send to the LLM
    /// * `json_mode` - Whether to request JSON structured output (true) or plain text (false)
    ///
    /// When `json_mode` is true, the response should be valid JSON matching the QueryResponse schema.
    /// When `json_mode` is false, the response can be plain text (used for answer generation).
    async fn complete(&self, prompt: &str, json_mode: bool) -> Result<String>;

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
        "groq" => Ok(Box::new(groq::GroqProvider::new(api_key, model)?)),
        _ => anyhow::bail!(
            "Unknown provider: {}. Supported: openai, anthropic, groq",
            provider_name
        ),
    }
}

/// Get the optimal model for page summarization based on provider and user model
///
/// Returns None if the same model should be used (no hybrid approach needed).
/// Returns Some(model_name) if a different, cheaper/higher-TPM model should be used for summaries.
///
/// Strategy:
/// - OpenAI: Use GPT-5-mini for summaries when user has GPT-5.1 (30K TPM limit)
/// - Anthropic: Use Claude Haiku-4.5 for summaries when user has Opus/Sonnet
/// - Groq: Always None (all models have 300K TPM, no hybrid needed)
pub fn get_summary_model(provider: &str, user_model: &str) -> Option<String> {
    match provider.to_lowercase().as_str() {
        "openai" => {
            // Use GPT-5-mini for summaries if user has low-TPM model
            if user_model.contains("gpt-5.1") || user_model.contains("gpt-5-turbo") {
                Some("gpt-5-mini".to_string())
            } else if user_model.contains("gpt-5-mini") {
                None // Already using mini, same model is fine
            } else {
                // For other/older models, use mini as safe default
                Some("gpt-5-mini".to_string())
            }
        }
        "anthropic" => {
            // Use Haiku-4.5 unless already using it
            if user_model.contains("haiku-4.5") {
                None // Already using Haiku
            } else {
                Some("claude-haiku-4.5".to_string())
            }
        }
        "groq" => {
            // 300K TPM for all Groq models - no hybrid needed
            None
        }
        _ => None, // Unknown provider, use same model
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
        if let Err(e) = provider {
            assert!(e.to_string().contains("Unknown provider"));
        }
    }
}
