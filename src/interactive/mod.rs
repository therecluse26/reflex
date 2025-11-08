// Interactive mode modules
mod app;
mod effects;
mod history;
mod input;
mod mouse;
mod results;
mod terminal;
mod theme;
mod ui;

use anyhow::Result;
use app::InteractiveApp;

/// Main entry point for interactive mode
/// Launches the TUI and runs the event loop
pub fn run_interactive() -> Result<()> {
    let mut app = InteractiveApp::new()?;
    app.run()
}
