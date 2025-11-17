//! Configuration for semantic query feature

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

/// Semantic query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConfig {
    /// Enable semantic query feature
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// LLM provider (openai, anthropic, gemini, groq)
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Optional model override (uses provider default if None)
    #[serde(default)]
    pub model: Option<String>,

    /// Auto-execute generated commands without confirmation
    #[serde(default)]
    pub auto_execute: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_provider() -> String {
    "openai".to_string()
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "openai".to_string(),
            model: None,
            auto_execute: false,
        }
    }
}

/// Load semantic config from .reflex/config.toml
///
/// Falls back to defaults if file doesn't exist or [semantic] section is missing.
pub fn load_config(cache_dir: &Path) -> Result<SemanticConfig> {
    let config_path = cache_dir.join("config.toml");

    if !config_path.exists() {
        log::debug!("No config.toml found, using default semantic config");
        return Ok(SemanticConfig::default());
    }

    let config_str = std::fs::read_to_string(&config_path)
        .context("Failed to read config.toml")?;

    let toml_value: toml::Value = toml::from_str(&config_str)
        .context("Failed to parse config.toml")?;

    // Extract [semantic] section
    if let Some(semantic_table) = toml_value.get("semantic") {
        let config: SemanticConfig = semantic_table.clone().try_into()
            .context("Failed to parse [semantic] section")?;
        Ok(config)
    } else {
        log::debug!("No [semantic] section in config.toml, using defaults");
        Ok(SemanticConfig::default())
    }
}

/// User configuration structure for ~/.reflex/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserConfig {
    #[serde(default)]
    credentials: Option<Credentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Credentials {
    #[serde(default)]
    openai_api_key: Option<String>,
    #[serde(default)]
    anthropic_api_key: Option<String>,
    #[serde(default)]
    gemini_api_key: Option<String>,
    #[serde(default)]
    groq_api_key: Option<String>,
    #[serde(default)]
    openai_model: Option<String>,
    #[serde(default)]
    anthropic_model: Option<String>,
    #[serde(default)]
    gemini_model: Option<String>,
    #[serde(default)]
    groq_model: Option<String>,
}

/// Load user configuration from ~/.reflex/config.toml
fn load_user_config() -> Result<Option<UserConfig>> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            log::debug!("Could not determine home directory");
            return Ok(None);
        }
    };

    let config_path = home.join(".reflex").join("config.toml");

    if !config_path.exists() {
        log::debug!("No user config found at ~/.reflex/config.toml");
        return Ok(None);
    }

    let config_str = std::fs::read_to_string(&config_path)
        .context("Failed to read ~/.reflex/config.toml")?;

    let config: UserConfig = toml::from_str(&config_str)
        .context("Failed to parse ~/.reflex/config.toml")?;

    Ok(Some(config))
}

/// Get API key for a provider
///
/// Checks in priority order:
/// 1. ~/.reflex/config.toml (user config file)
/// 2. {PROVIDER}_API_KEY environment variable (e.g., OPENAI_API_KEY)
/// 3. Error if not found
pub fn get_api_key(provider: &str) -> Result<String> {
    // First check user config file
    if let Ok(Some(user_config)) = load_user_config() {
        if let Some(credentials) = &user_config.credentials {
            // Get the appropriate key based on provider
            let key = match provider.to_lowercase().as_str() {
                "openai" => credentials.openai_api_key.as_ref(),
                "anthropic" => credentials.anthropic_api_key.as_ref(),
                "gemini" => credentials.gemini_api_key.as_ref(),
                "groq" => credentials.groq_api_key.as_ref(),
                _ => None,
            };

            if let Some(api_key) = key {
                log::debug!("Using {} API key from ~/.reflex/config.toml", provider);
                return Ok(api_key.clone());
            }
        }
    }

    // Fall back to environment variables
    let env_var = match provider.to_lowercase().as_str() {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "gemini" => "GEMINI_API_KEY",
        "groq" => "GROQ_API_KEY",
        _ => anyhow::bail!("Unknown provider: {}", provider),
    };

    env::var(env_var).with_context(|| {
        format!(
            "API key not found for provider '{}'.\n\
             \n\
             Either:\n\
             1. Run 'rfx ask --configure' to set up your API key interactively\n\
             2. Set the {} environment variable manually\n\
             \n\
             Example: export {}=sk-...",
            provider, env_var, env_var
        )
    })
}

/// Get the preferred model for a provider from user config
///
/// Returns None if no model is configured for this provider.
/// The caller should use provider defaults if None is returned.
pub fn get_user_model(provider: &str) -> Option<String> {
    if let Ok(Some(user_config)) = load_user_config() {
        if let Some(credentials) = &user_config.credentials {
            let model = match provider.to_lowercase().as_str() {
                "openai" => credentials.openai_model.as_ref(),
                "anthropic" => credentials.anthropic_model.as_ref(),
                "gemini" => credentials.gemini_model.as_ref(),
                "groq" => credentials.groq_model.as_ref(),
                _ => None,
            };

            if let Some(model_name) = model {
                log::debug!("Using {} model from ~/.reflex/config.toml: {}", provider, model_name);
                return Some(model_name.clone());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = SemanticConfig::default();
        assert_eq!(config.enabled, true);
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, None);
        assert_eq!(config.auto_execute, false);
    }

    #[test]
    fn test_load_config_no_file() {
        let temp = TempDir::new().unwrap();
        let config = load_config(temp.path()).unwrap();

        // Should return defaults
        assert_eq!(config.provider, "openai");
        assert_eq!(config.enabled, true);
    }

    #[test]
    fn test_load_config_with_semantic_section() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[semantic]
enabled = true
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
auto_execute = true
            "#,
        )
        .unwrap();

        let config = load_config(temp.path()).unwrap();
        assert_eq!(config.enabled, true);
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, Some("claude-3-5-sonnet-20241022".to_string()));
        assert_eq!(config.auto_execute, true);
    }

    #[test]
    fn test_load_config_without_semantic_section() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[index]
languages = []
            "#,
        )
        .unwrap();

        let config = load_config(temp.path()).unwrap();
        // Should return defaults
        assert_eq!(config.provider, "openai");
    }

    #[test]
    fn test_get_api_key_env_var() {
        env::set_var("OPENAI_API_KEY", "test-key-123");
        let key = get_api_key("openai").unwrap();
        assert_eq!(key, "test-key-123");
        env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn test_get_api_key_missing() {
        env::remove_var("GROQ_API_KEY");
        let result = get_api_key("groq");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("GROQ_API_KEY"));
    }

    #[test]
    fn test_get_api_key_unknown_provider() {
        let result = get_api_key("unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown provider"));
    }
}
