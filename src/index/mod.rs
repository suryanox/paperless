use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT, Value};
use tantivy::snippet::SnippetGenerator;
use tantivy::{Index, TantivyDocument, doc};

use crate::pdf::ExtractedDoc;

#[derive(Clone, Copy)]
struct Fields {
    path: Field,
    filename: Field,
    body: Field,
    size: Field,
    mtime: Field,
}

impl Fields {
    fn from_schema(schema: &Schema) -> Result<Self> {
        Ok(Self {
            path: schema.get_field("path")?,
            filename: schema.get_field("filename")?,
            body: schema.get_field("body")?,
            size: schema.get_field("size")?,
            mtime: schema.get_field("mtime")?,
        })
    }
}

fn build_schema() -> Schema {
    let mut b = Schema::builder();
    // STRING = indexed as one raw token (exact match, good for lookup/delete by path)
    b.add_text_field("path", STRING | STORED);
    // TEXT = tokenized + lowercased, so it's searchable word by word
    b.add_text_field("filename", TEXT | STORED);
    b.add_text_field("body", TEXT | STORED);
    b.add_u64_field("size", STORED);
    b.add_u64_field("mtime", STORED);
    b.build()
}

pub fn index_dir() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("", "", "paperless").context("could not determine data directory")?;
    Ok(dirs.data_dir().join("index"))
}

/// Creates a fresh index on disk. Fails if one exists unless `force` is set.
pub fn create_index(force: bool) -> Result<Index> {
    let dir = index_dir()?;

    if dir.exists() {
        if !force {
            bail!(
                "index already exists at {}\nuse `paperless init --force` to rebuild it",
                dir.display()
            );
        }
        fs::remove_dir_all(&dir)?;
    }

    fs::create_dir_all(&dir)?;
    Ok(Index::create_in_dir(&dir, build_schema())?)
}

pub fn write_docs(index: &Index, docs: &[ExtractedDoc]) -> Result<()> {
    let f = Fields::from_schema(&index.schema())?;
    let mut writer = index.writer(100_000_000)?; // 100MB heap budget

    for d in docs {
        let path = d.file.path.to_string_lossy().into_owned();
        let filename = d
            .file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mtime = d
            .file
            .mtime
            .duration_since(UNIX_EPOCH)
            .map(|t| t.as_secs())
            .unwrap_or(0);

        writer.add_document(doc!(
            f.path => path,
            f.filename => filename,
            f.body => d.text.as_str(),
            f.size => d.file.size,
            f.mtime => mtime,
        ))?;
    }

    writer.commit()?;
    Ok(())
}

pub struct Hit {
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

pub fn search(query_str: &str, limit: usize) -> Result<Vec<Hit>> {
    let dir = index_dir()?;
    if !dir.exists() {
        bail!("no index found, run `paperless init` first");
    }

    let index = Index::open_in_dir(&dir)?;
    let f = Fields::from_schema(&index.schema())?;
    let searcher = index.reader()?.searcher();

    let mut parser = QueryParser::for_index(&index, vec![f.filename, f.body]);
    parser.set_conjunction_by_default(); // every word must match
    parser.set_field_boost(f.filename, 3.0); // filename hits rank higher
    // (field, prefix, max edit distance, transposition counts as 1 edit)
    parser.set_field_fuzzy(f.filename, false, 1, true);
    parser.set_field_fuzzy(f.body, false, 1, true);

    // Lenient: odd characters in user input won't turn into a hard error
    let (query, _errors) = parser.parse_query_lenient(query_str);

    let top = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;

    let mut snippets = SnippetGenerator::create(&searcher, &*query, f.body)?;
    snippets.set_max_num_chars(160);

    let mut hits = Vec::with_capacity(top.len());
    for (score, addr) in top {
        let doc: TantivyDocument = searcher.doc(addr)?;

        let path = doc
            .get_first(f.path)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let snippet = snippets
            .snippet_from_doc(&doc)
            .fragment()
            .replace('\n', " ");

        hits.push(Hit {
            path,
            score,
            snippet,
        });
    }

    Ok(hits)
}
