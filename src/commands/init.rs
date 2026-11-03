use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use directories::UserDirs;

use crate::index::{self, Meta};
use crate::{pdf, ui, walker};

pub fn run(mut paths: Vec<PathBuf>, force: bool) -> Result<()> {
    let start = Instant::now();
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

    let sp = ui::spinner("Scanning for PDFs…");
    let pdfs = walker::find_pdfs(&roots);
    sp.finish_with_message(format!("Found {} PDFs", pdfs.len()));

    let total_bytes: u64 = pdfs.iter().map(|p| p.size).sum();
    let (docs, stats) = pdf::extract_all(pdfs);

    let sp = ui::spinner("Writing index…");
    index::write_docs(&idx, &docs)?;
    index::save_meta(&Meta { roots })?;
    sp.finish_and_clear();

    ui::print_init_summary(&stats, total_bytes, start.elapsed());
    Ok(())
}
