use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Debug, Parser)]
#[command(
    name = "paperless",
    version,
    about = "Fuzzy search for all the PDFs on your machine",
    styles = STYLES,
    after_help = "Examples:\n  paperless init --path ~/Documents\n  paperless search tax return 2023\n  paperless search invoice --json | jq '.[].path'"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init {
        #[arg(short, long)]
        path: Vec<PathBuf>,

        #[arg(short, long)]
        force: bool,
    },

    Search {
        #[arg(required = true)]
        query: Vec<String>,

        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        #[arg(long)]
        json: bool,
    },
    Update {
        #[arg(short, long)]
        path: Vec<PathBuf>,
    },
}
