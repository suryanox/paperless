use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::walker::PdfFile;
use crate::{index, pdf, walker};

struct Diff {
    to_add: Vec<PdfFile>,
    to_update: Vec<PdfFile>,
    to_remove: Vec<String>,
    unchanged: usize,
}

/// Pure logic, no tantivy, easy to unit test.
fn diff(found: Vec<PdfFile>, indexed: &HashMap<String, (u64, u64)>, roots: &[PathBuf]) -> Diff {
    let mut to_add = Vec::new();
    let mut to_update = Vec::new();
    let mut unchanged = 0;
    let mut seen: HashSet<String> = HashSet::with_capacity(found.len());

    for file in found {
        let key = file.path.to_string_lossy().into_owned();
        match indexed.get(&key) {
            None => to_add.push(file),
            Some(&(size, mtime)) if size != file.size || mtime != file.mtime_secs() => {
                to_update.push(file)
            }
            Some(_) => unchanged += 1,
        }
        seen.insert(key);
    }

    // Only remove entries under the roots we actually scanned, so a narrower
    // `--path` override never wipes the rest of the index.
    let to_remove = indexed
        .keys()
        .filter(|k| !seen.contains(*k))
        .filter(|k| roots.iter().any(|r| Path::new(k.as_str()).starts_with(r)))
        .cloned()
        .collect();

    Diff {
        to_add,
        to_update,
        to_remove,
        unchanged,
    }
}

pub fn run(path_override: Vec<PathBuf>) -> Result<()> {
    let idx = index::open_index()?;

    let roots = if path_override.is_empty() {
        index::load_meta()?.roots
    } else {
        path_override
            .into_iter()
            .map(|p| p.canonicalize())
            .collect::<std::io::Result<Vec<_>>>()?
    };

    let sp = crate::ui::spinner("Scanning for changes…");
    let found = walker::find_pdfs(&roots);
    let indexed = index::indexed_files(&idx)?;
    let d = diff(found, &indexed, &roots);
    sp.finish_and_clear();

    let (n_add, n_upd, n_rem, unchanged) = (
        d.to_add.len(),
        d.to_update.len(),
        d.to_remove.len(),
        d.unchanged,
    );

    if n_add + n_upd + n_rem == 0 {
        crate::ui::up_to_date(unchanged);
        return Ok(());
    }

    let mut remove = d.to_remove;
    remove.extend(
        d.to_update
            .iter()
            .map(|f| f.path.to_string_lossy().into_owned()),
    );
    let to_extract: Vec<PdfFile> = d.to_add.into_iter().chain(d.to_update).collect();

    let (docs, stats) = pdf::extract_all(to_extract);
    index::apply_changes(&idx, &remove, &docs)?;

    crate::ui::print_update_summary(n_add, n_upd, n_rem, unchanged, stats.failed);
    Ok(())
}
