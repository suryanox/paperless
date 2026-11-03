use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "paperless", version, about = "Fuzzy search for all the PDFs on your machine")]
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
    },
}