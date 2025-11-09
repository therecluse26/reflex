//! Reflex CLI entrypoint

use clap::Parser;

use reflex::cli::Cli;
use reflex::output;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = cli.execute() {
        // Display error in red with clean formatting
        output::error(&format!("Error: {:#}", e));
        std::process::exit(1);
    }
}
