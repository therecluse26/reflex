use std::env;

/// Terminal capabilities and feature detection
#[derive(Debug, Clone)]
pub struct TerminalCapabilities {
    /// Whether the terminal supports OSC 8 hyperlinks
    pub supports_hyperlinks: bool,
    /// Terminal type identifier
    pub terminal_type: TerminalType,
    /// Whether mouse events are supported
    pub supports_mouse: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminalType {
    ITerm2,
    WezTerm,
    Kitty,
    VSCode,
    Alacritty,
    Tmux,
    Screen,
    Unknown,
}

impl TerminalCapabilities {
    /// Detect terminal capabilities from environment variables
    pub fn detect() -> Self {
        let terminal_type = Self::detect_terminal_type();
        let supports_hyperlinks = Self::detect_hyperlink_support(&terminal_type);
        let supports_mouse = true; // Most modern terminals support mouse events

        Self {
            supports_hyperlinks,
            terminal_type,
            supports_mouse,
        }
    }

    fn detect_terminal_type() -> TerminalType {
        // Check TERM_PROGRAM first (most specific)
        if let Ok(term_program) = env::var("TERM_PROGRAM") {
            return match term_program.as_str() {
                "iTerm.app" => TerminalType::ITerm2,
                "WezTerm" => TerminalType::WezTerm,
                "vscode" => TerminalType::VSCode,
                _ => Self::detect_from_term_var(),
            };
        }

        Self::detect_from_term_var()
    }

    fn detect_from_term_var() -> TerminalType {
        if let Ok(term) = env::var("TERM") {
            if term.contains("kitty") {
                return TerminalType::Kitty;
            } else if term.contains("alacritty") {
                return TerminalType::Alacritty;
            } else if term.contains("tmux") {
                return TerminalType::Tmux;
            } else if term.contains("screen") {
                return TerminalType::Screen;
            }
        }

        TerminalType::Unknown
    }

    fn detect_hyperlink_support(terminal_type: &TerminalType) -> bool {
        matches!(
            terminal_type,
            TerminalType::ITerm2
                | TerminalType::WezTerm
                | TerminalType::Kitty
                | TerminalType::VSCode
        )
    }

    /// Generate an OSC 8 hyperlink for a file path
    pub fn make_hyperlink(&self, path: &str, line: usize, display_text: &str) -> String {
        if !self.supports_hyperlinks {
            return display_text.to_string();
        }

        let url = format!("file://{path}:{line}");
        format!("\x1b]8;;{url}\x1b\\{display_text}\x1b]8;;\x1b\\")
    }

    /// Get the appropriate hint text for opening files
    pub fn open_hint(&self) -> &'static str {
        if self.supports_hyperlinks {
            "[Cmd+Click to open]"
        } else {
            "[Press 'o' to open]"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_detection() {
        let caps = TerminalCapabilities::detect();
        assert!(caps.supports_mouse);
    }

    #[test]
    fn test_hyperlink_generation() {
        let caps = TerminalCapabilities {
            supports_hyperlinks: true,
            terminal_type: TerminalType::Kitty,
            supports_mouse: true,
        };

        let link = caps.make_hyperlink("/path/to/file.rs", 42, "file.rs:42");
        assert!(link.contains("file:///path/to/file.rs:42"));
    }
}
