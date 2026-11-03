mod cli;
mod commands;
mod error;
mod index;
mod pdf;
mod walker;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path, force } => commands::init::run(path, force)?,
        Commands::Search { query, limit } => commands::search::run(query.join(" "), limit)?,
        Commands::Update { path } => commands::update::run(path)?,
    }

    Ok(())
}
