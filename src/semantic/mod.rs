//! Semantic query generation using LLMs

pub mod config;
pub mod configure;
pub mod context;
pub mod executor;
pub mod prompt;
pub mod providers;
pub mod schema;

// Re-export main types for convenience
pub use configure::run_configure_wizard;
pub use executor::{execute_queries, parse_command, ParsedCommand};
pub use schema::{QueryCommand, QueryResponse as SemanticQueryResponse};

use anyhow::{Context, Result};
use crate::cache::CacheManager;

/// Generate query commands from a natural language question
///
/// This is the main entry point for the semantic query feature.
pub async fn ask_question(
    question: &str,
    cache: &CacheManager,
    provider_override: Option<String>,
) -> Result<schema::QueryResponse> {
    // Load config
    let mut config = config::load_config(cache.path())?;

    // Override provider if specified
    if let Some(provider) = provider_override {
        config.provider = provider;
    }

    // Get API key
    let api_key = config::get_api_key(&config.provider)?;

    // Determine which model to use (priority order):
    // 1. Project config model override (config.model from .reflex/config.toml)
    // 2. User-configured model for this provider (~/.reflex/config.toml)
    // 3. Provider default (handled by provider)
    let model = if config.model.is_some() {
        config.model.clone()
    } else {
        config::get_user_model(&config.provider)
    };

    // Create provider
    let provider = providers::create_provider(
        &config.provider,
        api_key,
        model,
    )?;

    log::info!("Using provider: {} (model: {})", provider.name(), provider.default_model());

    // Build prompt with language injection
    let prompt = prompt::build_prompt(question, cache)?;

    log::debug!("Generated prompt ({} chars)", prompt.len());

    // Call LLM with retry logic
    let json_response = call_with_retry(&*provider, &prompt, 2).await?;

    log::debug!("Received response ({} chars)", json_response.len());

    // Parse JSON response
    let response: schema::QueryResponse = serde_json::from_str(&json_response)
        .context("Failed to parse LLM response as JSON. The LLM may have returned invalid JSON.")?;

    // Validate response
    if response.queries.is_empty() {
        anyhow::bail!("LLM returned no queries");
    }

    log::info!("Generated {} quer{}", response.queries.len(), if response.queries.len() == 1 { "y" } else { "ies" });

    Ok(response)
}

/// Strip markdown code fences from LLM response
///
/// Some LLMs (especially Claude) wrap JSON in markdown code fences
/// even when explicitly instructed not to. This function removes them.
///
/// Handles:
/// - ```json\n{...}\n```
/// - ```\n{...}\n```
/// - {raw JSON} (no-op, returns as-is)
fn strip_markdown_fences(text: &str) -> &str {
    let trimmed = text.trim();

    // Check for markdown code fence pattern
    if trimmed.starts_with("```") && trimmed.ends_with("```") {
        // Remove opening fence (either ```json or just ```)
        let without_start = if let Some(rest) = trimmed.strip_prefix("```json") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("```") {
            rest
        } else {
            return trimmed;
        };

        // Remove closing fence
        let without_end = without_start.strip_suffix("```")
            .unwrap_or(without_start);

        without_end.trim()
    } else {
        trimmed
    }
}

/// Call LLM provider with retry logic
///
/// Retries up to `max_retries` times on:
/// - Network errors
/// - Invalid JSON responses
///
/// Uses exponential backoff between retries.
async fn call_with_retry(
    provider: &dyn providers::LlmProvider,
    prompt: &str,
    max_retries: usize,
) -> Result<String> {
    let mut last_error = None;

    for attempt in 0..=max_retries {
        if attempt > 0 {
            log::warn!("Retrying LLM call (attempt {}/{})", attempt + 1, max_retries + 1);
        }

        match provider.complete(prompt).await {
            Ok(response) => {
                // Strip markdown code fences (Claude often adds them despite instructions)
                let cleaned_response = strip_markdown_fences(&response);

                // Validate that response is valid JSON for our schema
                match serde_json::from_str::<schema::QueryResponse>(cleaned_response) {
                    Ok(_) => {
                        // Valid response - return the cleaned version
                        return Ok(cleaned_response.to_string());
                    }
                    Err(e) => {
                        if attempt < max_retries {
                            log::warn!(
                                "Invalid JSON response from LLM, retrying ({}/{}): {}",
                                attempt + 1,
                                max_retries,
                                e
                            );
                            last_error = Some(anyhow::anyhow!(
                                "Invalid JSON format: {}. Response: {}",
                                e,
                                cleaned_response
                            ));

                            // Exponential backoff: 500ms, 1s, 1.5s...
                            let delay_ms = 500 * (attempt as u64 + 1);
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        } else {
                            // Final attempt failed
                            last_error = Some(anyhow::anyhow!(
                                "Invalid JSON format after {} attempts: {}. Response: {}",
                                max_retries + 1,
                                e,
                                cleaned_response
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                if attempt < max_retries {
                    log::warn!(
                        "LLM API call failed, retrying ({}/{}): {}",
                        attempt + 1,
                        max_retries,
                        e
                    );

                    // Exponential backoff
                    let delay_ms = 500 * (attempt as u64 + 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_fences_with_json_label() {
        let input = r#"```json
{
  "queries": [
    {
      "command": "query \"User\" --symbols --kind class --lang php",
      "order": 1,
      "merge": true
    }
  ]
}
```"#;
        let expected = r#"{
  "queries": [
    {
      "command": "query \"User\" --symbols --kind class --lang php",
      "order": 1,
      "merge": true
    }
  ]
}"#;
        assert_eq!(strip_markdown_fences(input), expected);
    }

    #[test]
    fn test_strip_markdown_fences_without_json_label() {
        let input = r#"```
{"queries": []}
```"#;
        let expected = r#"{"queries": []}"#;
        assert_eq!(strip_markdown_fences(input), expected);
    }

    #[test]
    fn test_strip_markdown_fences_no_fences() {
        let input = r#"{"queries": []}"#;
        assert_eq!(strip_markdown_fences(input), input);
    }

    #[test]
    fn test_strip_markdown_fences_with_whitespace() {
        let input = r#"  ```json
{"queries": []}
```  "#;
        let expected = r#"{"queries": []}"#;
        assert_eq!(strip_markdown_fences(input), expected);
    }

    #[test]
    fn test_module_structure() {
        // Just verify the module compiles
        assert!(true);
    }
}
