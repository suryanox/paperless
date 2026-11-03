use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::UserDirs;

use crate::walker;

pub fn run(mut paths: Vec<PathBuf>, _force: bool) -> Result<()> {
    if paths.is_empty() {
        let home = UserDirs::new()
            .context("could not determine home directory")?
            .home_dir()
            .to_path_buf();
        paths.push(home);
    }

    let pdfs = walker::find_pdfs(&paths);

    println!("Found {} PDFs", pdfs.len());
    for pdf in pdfs.iter().take(10) {
        println!("  {} ({} bytes)", pdf.path.display(), pdf.size);
    }

    Ok(())
}
