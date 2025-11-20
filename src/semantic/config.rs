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

    /// LLM provider (openai, anthropic, groq)
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Optional model override (uses provider default if None)
    #[serde(default)]
    pub model: Option<String>,

    /// Auto-execute generated commands without confirmation
    #[serde(default)]
    pub auto_execute: bool,

    /// Enable agentic mode (multi-step reasoning with context gathering)
    #[serde(default = "default_agentic_enabled")]
    pub agentic_enabled: bool,

    /// Maximum iterations for query refinement in agentic mode
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Maximum tool calls per context gathering phase
    #[serde(default = "default_max_tools")]
    pub max_tools_per_phase: usize,

    /// Enable result evaluation in agentic mode
    #[serde(default = "default_evaluation_enabled")]
    pub evaluation_enabled: bool,

    /// Evaluation strictness (0.0-1.0, higher is stricter)
    #[serde(default = "default_strictness")]
    pub evaluation_strictness: f32,
}

fn default_enabled() -> bool {
    true
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_agentic_enabled() -> bool {
    false // Disabled by default, opt-in for experimental feature
}

fn default_max_iterations() -> usize {
    2
}

fn default_max_tools() -> usize {
    5
}

fn default_evaluation_enabled() -> bool {
    true
}

fn default_strictness() -> f32 {
    0.5
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "openai".to_string(),
            model: None,
            auto_execute: false,
            agentic_enabled: false,
            max_iterations: 2,
            max_tools_per_phase: 5,
            evaluation_enabled: true,
            evaluation_strictness: 0.5,
        }
    }
}

/// Load semantic config from ~/.reflex/config.toml
///
/// Semantic configuration is ALWAYS user-level (not project-level).
/// Falls back to defaults if file doesn't exist or [semantic] section is missing.
///
/// Note: The cache_dir parameter is ignored - kept for API compatibility but will be removed in future.
pub fn load_config(_cache_dir: &Path) -> Result<SemanticConfig> {
    // Semantic config is always in user home directory, not project directory
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            log::debug!("Could not determine home directory, using defaults");
            return Ok(SemanticConfig::default());
        }
    };

    let config_path = home.join(".reflex").join("config.toml");

    if !config_path.exists() {
        log::debug!("No ~/.reflex/config.toml found, using default semantic config");
        return Ok(SemanticConfig::default());
    }

    let config_str = std::fs::read_to_string(&config_path)
        .context("Failed to read ~/.reflex/config.toml")?;

    let toml_value: toml::Value = toml::from_str(&config_str)
        .context("Failed to parse ~/.reflex/config.toml")?;

    // Extract [semantic] section
    if let Some(semantic_table) = toml_value.get("semantic") {
        let config: SemanticConfig = semantic_table.clone().try_into()
            .context("Failed to parse [semantic] section in ~/.reflex/config.toml")?;
        log::debug!("Loaded semantic config from ~/.reflex/config.toml: provider={}", config.provider);
        Ok(config)
    } else {
        log::debug!("No [semantic] section in ~/.reflex/config.toml, using defaults");
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
    groq_api_key: Option<String>,
    #[serde(default)]
    openai_model: Option<String>,
    #[serde(default)]
    anthropic_model: Option<String>,
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

/// Check if any API key is configured for any supported provider
///
/// Checks in priority order:
/// 1. ~/.reflex/config.toml (credentials section)
/// 2. Environment variables (OPENAI_API_KEY, ANTHROPIC_API_KEY, GROQ_API_KEY)
///
/// Returns true if at least one API key is found for any provider.
pub fn is_any_api_key_configured() -> bool {
    let providers = ["openai", "anthropic", "groq"];

    // Check user config file first
    if let Ok(Some(user_config)) = load_user_config() {
        if let Some(credentials) = &user_config.credentials {
            // Check if any provider has an API key in the config file
            if credentials.openai_api_key.is_some()
                || credentials.anthropic_api_key.is_some()
                || credentials.groq_api_key.is_some()
            {
                log::debug!("Found API key in ~/.reflex/config.toml");
                return true;
            }
        }
    }

    // Check environment variables
    for provider in &providers {
        let env_var = match *provider {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "groq" => "GROQ_API_KEY",
            _ => continue,
        };

        if env::var(env_var).is_ok() {
            log::debug!("Found {} environment variable", env_var);
            return true;
        }
    }

    log::debug!("No API keys found in config or environment variables");
    false
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

/// Save user's provider/model preference to ~/.reflex/config.toml
///
/// Updates the [credentials] section with the new model for the specified provider.
/// Creates the config file and directory if they don't exist.
pub fn save_user_provider(provider: &str, model: Option<&str>) -> Result<()> {
    let home = dirs::home_dir().context("Cannot find home directory")?;
    let config_dir = home.join(".reflex");
    let config_path = config_dir.join("config.toml");

    // Create directory if needed
    std::fs::create_dir_all(&config_dir)
        .context("Failed to create ~/.reflex directory")?;

    // Read existing config or create empty
    let mut config: toml::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read ~/.reflex/config.toml")?;
        toml::from_str(&content)
            .context("Failed to parse ~/.reflex/config.toml")?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    // Ensure [credentials] section exists
    let credentials = config
        .as_table_mut()
        .context("Config root is not a table")?
        .entry("credentials")
        .or_insert(toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .context("[credentials] is not a table")?;

    // Set model for this provider (if provided)
    if let Some(m) = model {
        let key = format!("{}_model", provider.to_lowercase());
        credentials.insert(key, toml::Value::String(m.to_string()));
        log::info!("Saved {} model: {}", provider, m);
    }

    // Write back to file
    let toml_str = toml::to_string_pretty(&config)
        .context("Failed to serialize config to TOML")?;
    std::fs::write(&config_path, toml_str)
        .context("Failed to write ~/.reflex/config.toml")?;

    Ok(())
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

        // Set HOME to temp directory to avoid loading user's config
        unsafe {
            env::set_var("HOME", temp.path());
        }
        let config = load_config(temp.path()).unwrap();
        unsafe {
            env::remove_var("HOME");
        }

        // Should return defaults
        assert_eq!(config.provider, "openai");
        assert_eq!(config.enabled, true);
    }

    #[test]
    fn test_load_config_with_semantic_section() {
        let temp = TempDir::new().unwrap();
        let reflex_dir = temp.path().join(".reflex");
        std::fs::create_dir_all(&reflex_dir).unwrap();
        let config_path = reflex_dir.join("config.toml");

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

        // Set HOME to temp directory to load test config
        unsafe {
            env::set_var("HOME", temp.path());
        }
        let config = load_config(temp.path()).unwrap();
        unsafe {
            env::remove_var("HOME");
        }

        assert_eq!(config.enabled, true);
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, Some("claude-3-5-sonnet-20241022".to_string()));
        assert_eq!(config.auto_execute, true);
    }

    #[test]
    fn test_load_config_without_semantic_section() {
        let temp = TempDir::new().unwrap();
        let reflex_dir = temp.path().join(".reflex");
        std::fs::create_dir_all(&reflex_dir).unwrap();
        let config_path = reflex_dir.join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[index]
languages = []
            "#,
        )
        .unwrap();

        // Set HOME to temp directory to load test config
        unsafe {
            env::set_var("HOME", temp.path());
        }
        let config = load_config(temp.path()).unwrap();
        unsafe {
            env::remove_var("HOME");
        }

        // Should return defaults
        assert_eq!(config.provider, "openai");
    }

    #[test]
    fn test_get_api_key_env_var() {
        let temp = TempDir::new().unwrap();

        // Set HOME to temp directory to avoid loading user's config
        unsafe {
            env::set_var("HOME", temp.path());
            env::set_var("OPENAI_API_KEY", "test-key-123");
        }

        let key = get_api_key("openai").unwrap();
        assert_eq!(key, "test-key-123");

        unsafe {
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_get_api_key_missing() {
        let temp = TempDir::new().unwrap();

        // Set HOME to temp directory to avoid loading user's config
        unsafe {
            env::set_var("HOME", temp.path());
            env::remove_var("GROQ_API_KEY");
        }

        let result = get_api_key("groq");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("GROQ_API_KEY"));

        unsafe {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_get_api_key_unknown_provider() {
        let result = get_api_key("unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown provider"));
    }
}
