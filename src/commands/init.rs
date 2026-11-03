use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::UserDirs;

use crate::{pdf, walker};

pub fn run(mut paths: Vec<PathBuf>, _force: bool) -> Result<()> {
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
        "Done: {} ok, {} failed, {} empty (likely scans)",
        stats.succeeded, stats.failed, stats.empty
    );

    if let Some(doc) = docs.iter().find(|d| !d.empty) {
        let preview: String = doc.text.chars().take(200).collect();
        println!("\nSample: {}\n{}", doc.file.path.display(), preview);
    }

    Ok(())
}
