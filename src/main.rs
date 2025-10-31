//! RefLex CLI entrypoint

use anyhow::Result;
use clap::Parser;

use reflex::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.execute()
}
