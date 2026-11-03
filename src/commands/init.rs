use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::UserDirs;

use crate::index::{self, Meta};
use crate::{pdf, walker};

pub fn run(mut paths: Vec<PathBuf>, force: bool) -> Result<()> {
    let idx = index::create_index(force)?;

    if paths.is_empty() {
        let home = UserDirs::new()
            .context("could not determine home directory")?
            .home_dir()
            .to_path_buf();
        paths.push(home);
    }

    let roots = paths
        .into_iter()
        .map(|p| {
            p.canonicalize()
                .with_context(|| format!("bad path: {}", p.display()))
        })
        .collect::<Result<Vec<_>>>()?;

    let pdfs = walker::find_pdfs(&roots);
    println!("Found {} PDFs, extracting text...", pdfs.len());

    let (docs, stats) = pdf::extract_all(pdfs);
    println!(
        "Extracted: {} ok, {} failed, {} empty (likely scans)",
        stats.succeeded, stats.failed, stats.empty
    );

    index::write_docs(&idx, &docs)?;
    index::save_meta(&Meta { roots })?;
    println!(
        "Indexed {} documents into {}",
        docs.len(),
        index::index_dir()?.display()
    );

    Ok(())
}
