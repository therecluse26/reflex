//! Interactive TUI configuration wizard for AI provider setup

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};

/// Available AI providers
const PROVIDERS: &[&str] = &["groq", "openai", "anthropic"];

/// Available models per provider
const OPENAI_MODELS: &[&str] = &[
    "gpt-5.1",
    "gpt-5.1-mini",
    "gpt-5.1-nano",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
];
const ANTHROPIC_MODELS: &[&str] = &[
    "claude-sonnet-4-5",
    "claude-haiku-4-5",
    "claude-sonnet-4",
];
const GROQ_MODELS: &[&str] = &[
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
    "meta-llama/llama-4-maverick-17b-128e-instruct",
    "meta-llama/llama-4-scout-17b-16e-instruct",
    "qwen/qwen3-32b",
    "moonshotai/kimi-k2-instruct-0905",
];

/// Wizard screen states
#[derive(Debug, Clone, PartialEq)]
enum WizardScreen {
    ProviderSelection,
    ApiKeyInput,
    ModelSelection,
    ConnectivityTest,
    Result { success: bool, message: String },
}

/// Load existing API key for a provider from ~/.reflex/config.toml
fn load_existing_api_key(provider: &str) -> Option<String> {
    match crate::semantic::config::get_api_key(provider) {
        Ok(key) => {
            log::debug!("Found existing API key for {}", provider);
            Some(key)
        }
        Err(_) => {
            log::debug!("No existing API key found for {}", provider);
            None
        }
    }
}

/// Mask API key for display (show first 7 and last 4 characters)
fn mask_api_key(key: &str) -> String {
    if key.len() <= 11 {
        // Too short to mask meaningfully
        return "*".repeat(key.len());
    }

    let start = &key[..7];
    let end = &key[key.len() - 4..];
    format!("{}...{}", start, end)
}

/// Check if a model is an OpenAI GPT-OSS model (requires special handling)
fn is_gpt_oss_model(model: &str) -> bool {
    model.starts_with("openai/gpt-oss-")
}

/// Main configuration wizard state
pub struct ConfigWizard {
    screen: WizardScreen,
    selected_provider_idx: usize,
    api_key: String,
    api_key_cursor: usize,
    selected_model_idx: usize,
    error_message: Option<String>,
    existing_api_key: Option<String>,
}

impl ConfigWizard {
    pub fn new() -> Self {
        Self {
            screen: WizardScreen::ProviderSelection,
            selected_provider_idx: 0,
            api_key: String::new(),
            api_key_cursor: 0,
            selected_model_idx: 0,
            error_message: None,
            existing_api_key: None,
        }
    }

    /// Get the currently selected provider
    fn selected_provider(&self) -> &str {
        PROVIDERS[self.selected_provider_idx]
    }

    /// Get available models for the current provider
    fn available_models(&self) -> &'static [&'static str] {
        match self.selected_provider() {
            "openai" => OPENAI_MODELS,
            "anthropic" => ANTHROPIC_MODELS,
            "groq" => GROQ_MODELS,
            _ => &[],
        }
    }

    /// Get the currently selected model
    fn selected_model(&self) -> &str {
        let models = self.available_models();
        models[self.selected_model_idx]
    }

    /// Handle keyboard input based on current screen
    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        // Handle Ctrl+C globally to exit wizard
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }

        match &self.screen {
            WizardScreen::ProviderSelection => self.handle_provider_selection_key(key),
            WizardScreen::ApiKeyInput => self.handle_api_key_input_key(key),
            WizardScreen::ModelSelection => self.handle_model_selection_key(key),
            WizardScreen::ConnectivityTest => Ok(false), // No input during test
            WizardScreen::Result { .. } => {
                // Any key exits on result screen
                if key.code == KeyCode::Enter || key.code == KeyCode::Char('q') {
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    /// Handle keys for provider selection screen
    fn handle_provider_selection_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_provider_idx > 0 {
                    self.selected_provider_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_provider_idx < PROVIDERS.len() - 1 {
                    self.selected_provider_idx += 1;
                }
            }
            KeyCode::Enter => {
                // Check if API key already exists for this provider
                self.existing_api_key = load_existing_api_key(self.selected_provider());

                // Move to API key input
                self.screen = WizardScreen::ApiKeyInput;
                self.api_key.clear();
                self.api_key_cursor = 0;
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                return Ok(true); // Exit wizard
            }
            _ => {}
        }
        Ok(false)
    }

    /// Handle keys for API key input screen
    fn handle_api_key_input_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.api_key.insert(self.api_key_cursor, c);
                self.api_key_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.api_key_cursor > 0 {
                    self.api_key_cursor -= 1;
                    self.api_key.remove(self.api_key_cursor);
                }
            }
            KeyCode::Delete => {
                if self.api_key_cursor < self.api_key.len() {
                    self.api_key.remove(self.api_key_cursor);
                }
            }
            KeyCode::Left => {
                if self.api_key_cursor > 0 {
                    self.api_key_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.api_key_cursor < self.api_key.len() {
                    self.api_key_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.api_key_cursor = 0;
            }
            KeyCode::End => {
                self.api_key_cursor = self.api_key.len();
            }
            KeyCode::Enter => {
                // If API key is empty and we have an existing one, keep the existing one
                if self.api_key.is_empty() {
                    if let Some(ref existing_key) = self.existing_api_key {
                        log::debug!("Keeping existing API key for {}", self.selected_provider());
                        self.api_key = existing_key.clone();
                        self.error_message = None;
                        self.selected_model_idx = 0;
                        self.screen = WizardScreen::ModelSelection;
                    } else {
                        self.error_message = Some("API key cannot be empty".to_string());
                    }
                } else {
                    // Move to model selection with new key
                    self.error_message = None;
                    self.selected_model_idx = 0;
                    self.screen = WizardScreen::ModelSelection;
                }
            }
            KeyCode::Esc => {
                // Go back to provider selection
                self.screen = WizardScreen::ProviderSelection;
            }
            _ => {}
        }
        Ok(false)
    }

    /// Handle keys for model selection screen
    fn handle_model_selection_key(&mut self, key: KeyEvent) -> Result<bool> {
        let models = self.available_models();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_model_idx > 0 {
                    self.selected_model_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_model_idx < models.len() - 1 {
                    self.selected_model_idx += 1;
                }
            }
            KeyCode::Enter => {
                // Move to connectivity test
                self.screen = WizardScreen::ConnectivityTest;
            }
            KeyCode::Esc => {
                // Go back to API key input
                self.screen = WizardScreen::ApiKeyInput;
            }
            _ => {}
        }
        Ok(false)
    }

    /// Render the current screen
    fn render(&self, frame: &mut Frame) {
        match &self.screen {
            WizardScreen::ProviderSelection => self.render_provider_selection(frame),
            WizardScreen::ApiKeyInput => self.render_api_key_input(frame),
            WizardScreen::ModelSelection => self.render_model_selection(frame),
            WizardScreen::ConnectivityTest => self.render_connectivity_test(frame),
            WizardScreen::Result { success, message } => {
                self.render_result(frame, *success, message)
            }
        }
    }

    /// Render provider selection screen
    fn render_provider_selection(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Title
        let title = Paragraph::new("Reflex AI Configuration Wizard")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Provider list
        let providers: Vec<ListItem> = PROVIDERS
            .iter()
            .enumerate()
            .map(|(idx, provider)| {
                let style = if idx == self.selected_provider_idx {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let prefix = if idx == self.selected_provider_idx {
                    "> "
                } else {
                    "  "
                };

                let provider_display = if *provider == "groq" {
                    format!("{} (recommended)", provider)
                } else {
                    provider.to_string()
                };

                ListItem::new(format!("{}{}", prefix, provider_display)).style(style)
            })
            .collect();

        let list = List::new(providers).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select AI Provider (↑/↓ to navigate, Enter to select, Esc/q/Ctrl+C to quit)"),
        );
        frame.render_widget(list, chunks[1]);

        // Help text
        let help = Paragraph::new("Use arrow keys or j/k to navigate, Enter to select, Esc/q/Ctrl+C to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }

    /// Render API key input screen
    fn render_api_key_input(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Title
        let title = Paragraph::new(format!(
            "Configure {} API Key",
            self.selected_provider()
        ))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // API key input (masked)
        let masked_key = "*".repeat(self.api_key.len());
        let input_text = if self.api_key_cursor < masked_key.len() {
            format!("{}█{}", &masked_key[..self.api_key_cursor], &masked_key[self.api_key_cursor..])
        } else {
            format!("{}█", masked_key)
        };

        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Enter API Key for {}", self.selected_provider())),
            );
        frame.render_widget(input, chunks[1]);

        // Error message or instructions
        let message_widget = if let Some(ref error) = self.error_message {
            Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
        } else if let Some(ref existing_key) = self.existing_api_key {
            // Show masked existing key
            let masked = mask_api_key(existing_key);
            Paragraph::new(format!(
                "Current API key: {}\n\
                Press Enter to keep existing key, or type a new key to replace it\n\
                Your API key will be securely stored in ~/.reflex/config.toml",
                masked
            ))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
        } else {
            Paragraph::new("Your API key will be securely stored in ~/.reflex/config.toml")
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Center)
        };
        frame.render_widget(message_widget, chunks[2]);

        // Help text
        let help = Paragraph::new("Enter to continue, Esc to go back, Ctrl+C to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[3]);
    }

    /// Render model selection screen
    fn render_model_selection(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Title
        let title = Paragraph::new(format!(
            "Select Model for {}",
            self.selected_provider()
        ))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Model list
        let models = self.available_models();
        let model_items: Vec<ListItem> = models
            .iter()
            .enumerate()
            .map(|(idx, model)| {
                let style = if idx == self.selected_model_idx {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let prefix = if idx == self.selected_model_idx {
                    "> "
                } else {
                    "  "
                };

                // Add recommended badge for first model
                let model_display = if idx == 0 {
                    format!("{} (recommended)", model)
                } else {
                    model.to_string()
                };

                ListItem::new(format!("{}{}", prefix, model_display)).style(style)
            })
            .collect();

        let list = List::new(model_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Model (↑/↓ to navigate, Enter to select, Esc to go back, Ctrl+C to quit)"),
        );
        frame.render_widget(list, chunks[1]);

        // Help text
        let help = Paragraph::new("Use arrow keys or j/k to navigate, Enter to select, Esc to go back, Ctrl+C to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }

    /// Render connectivity test screen
    fn render_connectivity_test(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(frame.area());

        // Title
        let title = Paragraph::new("Testing Connection...")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Loading message
        let message = Paragraph::new(format!(
            "Testing connection to {}...\n\nPlease wait...",
            self.selected_provider()
        ))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
        frame.render_widget(message, chunks[1]);
    }

    /// Render result screen
    fn render_result(&self, frame: &mut Frame, success: bool, message: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Title
        let title = if success {
            Paragraph::new("Configuration Successful!")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Paragraph::new("Configuration Failed")
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        };
        let title = title.alignment(Alignment::Center).block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Message
        let message_widget = Paragraph::new(message)
            .style(if success {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            })
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(message_widget, chunks[1]);

        // Help text
        let help = Paragraph::new(if success {
            "Press Enter, q, or Ctrl+C to exit"
        } else {
            "Press Enter, q, or Ctrl+C to exit (configuration not saved)"
        })
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }
}

/// Setup terminal for TUI
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("Failed to create terminal")
}

/// Restore terminal to normal mode
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;
    Ok(())
}

/// Run the configuration wizard
pub fn run_configure_wizard() -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut wizard = ConfigWizard::new();

    let result = run_wizard_loop(&mut terminal, &mut wizard);

    // Always restore terminal
    restore_terminal(&mut terminal)?;

    result
}

/// Main wizard event loop
fn run_wizard_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    wizard: &mut ConfigWizard,
) -> Result<()> {
    loop {
        // Render current screen
        terminal.draw(|frame| wizard.render(frame))?;

        // Handle connectivity test asynchronously
        if wizard.screen == WizardScreen::ConnectivityTest {
            let result = test_connectivity(wizard.selected_provider(), &wizard.api_key);
            match result {
                Ok(_) => {
                    // Save configuration
                    if let Err(e) = save_user_config(
                        wizard.selected_provider(),
                        &wizard.api_key,
                        wizard.selected_model(),
                    ) {
                        wizard.screen = WizardScreen::Result {
                            success: false,
                            message: format!("Failed to save configuration: {}", e),
                        };
                    } else {
                        wizard.screen = WizardScreen::Result {
                            success: true,
                            message: format!(
                                "Configuration saved successfully!\n\n\
                                Provider: {}\n\
                                Config file: ~/.reflex/config.toml\n\n\
                                You can now use 'rfx ask' to query your codebase.",
                                wizard.selected_provider()
                            ),
                        };
                    }
                }
                Err(e) => {
                    wizard.screen = WizardScreen::Result {
                        success: false,
                        message: format!(
                            "Connectivity test failed: {}\n\n\
                            Please check your API key and try again.",
                            e
                        ),
                    };
                }
            }
            continue;
        }

        // Handle keyboard input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let should_exit = wizard.handle_key(key)?;
                if should_exit {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Test connectivity to the selected provider
fn test_connectivity(provider_name: &str, api_key: &str) -> Result<()> {
    // Create a tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new()
        .context("Failed to create async runtime")?;

    runtime.block_on(async {
        // Create provider instance
        let provider = crate::semantic::providers::create_provider(
            provider_name,
            api_key.to_string(),
            None,
        )?;

        // Try to make a simple API call to test connectivity
        // Note: Must contain "json" for OpenAI/Groq structured output requirement
        let test_prompt = "Please respond with valid JSON: {\"status\": \"ok\"}";

        // Call complete method
        provider.complete(test_prompt, true).await?;  // json_mode: true for test

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

/// Save user configuration to ~/.reflex/config.toml
fn save_user_config(provider: &str, api_key: &str, model: &str) -> Result<()> {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs;

    #[derive(Debug, Serialize, Deserialize)]
    struct UserConfig {
        #[serde(default)]
        semantic: SemanticSection,
        #[serde(default)]
        credentials: HashMap<String, String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct SemanticSection {
        provider: String,
    }

    impl Default for SemanticSection {
        fn default() -> Self {
            Self {
                provider: "openai".to_string(),
            }
        }
    }

    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let config_dir = home.join(".reflex");
    fs::create_dir_all(&config_dir)
        .context("Failed to create ~/.reflex directory")?;

    let config_path = config_dir.join("config.toml");

    // Load existing config if it exists
    let mut config = if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)
            .context("Failed to read existing config file")?;
        toml::from_str::<UserConfig>(&config_str)
            .unwrap_or_else(|_| UserConfig {
                semantic: SemanticSection::default(),
                credentials: HashMap::new(),
            })
    } else {
        UserConfig {
            semantic: SemanticSection::default(),
            credentials: HashMap::new(),
        }
    };

    // Update the semantic section with selected provider
    config.semantic.provider = provider.to_string();

    // Update the specific provider's key and model in credentials
    let key_name = format!("{}_api_key", provider);
    let model_name = format!("{}_model", provider);
    config.credentials.insert(key_name, api_key.to_string());
    config.credentials.insert(model_name, model.to_string());

    // Serialize to TOML
    let toml_content = toml::to_string_pretty(&config)
        .context("Failed to serialize config to TOML")?;

    // Prepend comment header
    let final_content = format!(
        "# Reflex User Configuration\n\
         # This file stores your AI provider API keys\n\
         # Location: ~/.reflex/config.toml\n\
         \n\
         {}",
        toml_content
    );

    fs::write(&config_path, final_content)
        .context("Failed to write configuration file")?;

    log::info!("Configuration saved to {:?}", config_path);

    Ok(())
}
