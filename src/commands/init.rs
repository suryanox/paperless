use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::UserDirs;

use crate::{index, pdf, walker};

pub fn run(mut paths: Vec<PathBuf>, force: bool) -> Result<()> {
    // Fail fast if an index exists, before doing any slow work
    let idx = index::create_index(force)?;

    if paths.is_empty() {
        let home = UserDirs::new()
            .context("could not determine home directory")?
            .home_dir()
            .to_path_buf();
        paths.push(home);
    }

    let pdfs = walker::find_pdfs(&paths);
    println!("Found {} PDFs, extracting text...", pdfs.len());

    let (docs, stats) = pdf::extract_all(pdfs);
    println!(
        "Extracted: {} ok, {} failed, {} empty (likely scans)",
        stats.succeeded, stats.failed, stats.empty
    );

    index::write_docs(&idx, &docs)?;
    println!(
        "Indexed {} documents into {}",
        docs.len(),
        index::index_dir()?.display()
    );

    Ok(())
}
