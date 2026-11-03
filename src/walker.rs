use std::path::PathBuf;
use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone)]
pub struct PdfFile {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: SystemTime,
}

impl PdfFile {
    pub fn mtime_secs(&self) -> u64 {
        self.mtime
            .duration_since(UNIX_EPOCH)
            .map(|t| t.as_secs())
            .unwrap_or(0)
    }
}

const SKIP_DIRS: &[&str] = &["node_modules", "target", "proc", "sys", "dev"];

fn should_descend(entry: &DirEntry) -> bool {
    // Never prune the root the user explicitly asked for
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }

    let name = entry.file_name().to_string_lossy();

    // Hidden dirs (.git, .cache, ...) and known junk
    !(name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()))
}

fn is_pdf(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .path()
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
}

pub fn find_pdfs(roots: &[PathBuf]) -> Vec<PdfFile> {
    roots
        .iter()
        .flat_map(|root| {
            WalkDir::new(root)
                .into_iter()
                .filter_entry(should_descend)
                .filter_map(|e| e.ok())
                .filter(is_pdf)
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    Some(PdfFile {
                        size: meta.len(),
                        mtime: meta.modified().ok()?,
                        path: e.into_path(),
                    })
                })
        })
        .collect()
}
