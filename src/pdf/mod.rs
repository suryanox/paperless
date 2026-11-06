use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use indicatif::ProgressStyle;
use rayon::prelude::*;

use crate::error::ExtractError;
use crate::walker::PdfFile;

#[derive(Debug)]
pub struct ExtractedDoc {
    pub file: PdfFile,
    pub text: String,
    pub empty: bool,
}

#[derive(Debug, Default)]
pub struct ExtractStats {
    pub succeeded: usize,
    pub failed: usize,
    pub empty: usize,
}

pub fn extract_text(path: &Path) -> Result<String, ExtractError> {
    match panic::catch_unwind(AssertUnwindSafe(|| pdf_extract::extract_text(path))) {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(ExtractError::Pdf(e.to_string())),
        Err(_) => Err(ExtractError::Panicked),
    }
}

pub fn extract_all(files: Vec<PdfFile>) -> (Vec<ExtractedDoc>, ExtractStats) {
    let total = files.len();

    let pb = crate::ui::progress_bar(total as u64, "Extracting");
    pb.set_style(
        ProgressStyle::with_template("{bar:40.cyan/blue} {pos}/{len} [{elapsed_precise}] {msg}")
            .unwrap(),
    );

    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let docs: Vec<ExtractedDoc> = files
        .into_par_iter()
        .filter_map(|file| {
            let result = extract_text(&file.path);
            pb.inc(1);

            match result {
                Ok(text) => {
                    let empty = text.trim().is_empty();
                    Some(ExtractedDoc { file, text, empty })
                }
                Err(_) => None,
            }
        })
        .collect();

    panic::set_hook(prev_hook);
    pb.finish_and_clear();

    let stats = ExtractStats {
        succeeded: docs.len(),
        failed: total - docs.len(),
        empty: docs.iter().filter(|d| d.empty).count(),
    };

    (docs, stats)
}
