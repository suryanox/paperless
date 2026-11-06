mod cli;
mod commands;
mod error;
mod index;
mod pdf;
mod ui;
mod walker;

use clap::Parser;
use cli::{Cli, Commands};
use console::style;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {:#}", style("error:").for_stderr().red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path, force } => commands::init::run(path, force)?,
        Commands::Update { path } => commands::update::run(path)?,
        Commands::Search { query, limit, json } => {
            commands::search::run(query.join(" "), limit, json)?
        }
    }

    Ok(())
}
