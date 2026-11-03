use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use strsim::damerau_levenshtein;
use tantivy::collector::{DocSetCollector, TopDocs};
use tantivy::query::{AllQuery, QueryParser};
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT, Value};
use tantivy::{Index, TantivyDocument, Term, doc};

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
    b.add_text_field("path", STRING | STORED);
    b.add_text_field("filename", TEXT | STORED);
    b.add_text_field("body", TEXT | STORED);
    b.add_u64_field("size", STORED);
    b.add_u64_field("mtime", STORED);
    b.build()
}

fn data_dir() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("", "", "paperless").context("could not determine data directory")?;
    Ok(dirs.data_dir().to_path_buf())
}

pub fn index_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("index"))
}

/// Remembers what `init` scanned so `update` can reuse it.
#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
    pub roots: Vec<PathBuf>,
}

fn meta_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("meta.json"))
}

pub fn save_meta(meta: &Meta) -> Result<()> {
    fs::write(meta_path()?, serde_json::to_string_pretty(meta)?)?;
    Ok(())
}

pub fn load_meta() -> Result<Meta> {
    let raw = fs::read_to_string(meta_path()?)
        .context("no saved roots found, run `paperless init` first")?;
    Ok(serde_json::from_str(&raw)?)
}

// ---------- open / create ----------

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

pub fn open_index() -> Result<Index> {
    let dir = index_dir()?;
    if !dir.exists() {
        bail!("no index found, run `paperless init` first");
    }
    Ok(Index::open_in_dir(&dir)?)
}

// ---------- writing ----------

fn to_document(f: &Fields, d: &ExtractedDoc) -> TantivyDocument {
    let path = d.file.path.to_string_lossy().into_owned();
    let filename = d
        .file
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    doc!(
        f.path => path,
        f.filename => filename,
        f.body => d.text.as_str(),
        f.size => d.file.size,
        f.mtime => d.file.mtime_secs(),
    )
}

/// Deletes every doc whose path is in `remove_paths`, adds `docs`, one commit.
pub fn apply_changes(index: &Index, remove_paths: &[String], docs: &[ExtractedDoc]) -> Result<()> {
    let f = Fields::from_schema(&index.schema())?;
    let mut writer = index.writer(100_000_000)?;

    // Deletes must come BEFORE adds: a delete only affects docs added earlier
    // in the same batch, so this order keeps re-added changed files alive.
    for p in remove_paths {
        writer.delete_term(Term::from_field_text(f.path, p));
    }
    for d in docs {
        writer.add_document(to_document(&f, d))?;
    }

    writer.commit()?;
    Ok(())
}

pub fn write_docs(index: &Index, docs: &[ExtractedDoc]) -> Result<()> {
    apply_changes(index, &[], docs)
}

/// path -> (size, mtime_secs) for everything currently indexed.
pub fn indexed_files(index: &Index) -> Result<HashMap<String, (u64, u64)>> {
    let f = Fields::from_schema(&index.schema())?;
    let searcher = index.reader()?.searcher();
    let addrs = searcher.search(&AllQuery, &DocSetCollector)?;

    let mut map = HashMap::with_capacity(addrs.len());
    for addr in addrs {
        let doc: TantivyDocument = searcher.doc(addr)?;
        let path = doc
            .get_first(f.path)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let size = doc.get_first(f.size).and_then(|v| v.as_u64()).unwrap_or(0);
        let mtime = doc.get_first(f.mtime).and_then(|v| v.as_u64()).unwrap_or(0);
        map.insert(path, (size, mtime));
    }
    Ok(map)
}

pub struct Hit {
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

pub fn search(query_str: &str, limit: usize) -> Result<Vec<Hit>> {
    let index = open_index()?;
    let f = Fields::from_schema(&index.schema())?;
    let searcher = index.reader()?.searcher();

    let mut parser = QueryParser::for_index(&index, vec![f.filename, f.body]);
    parser.set_conjunction_by_default();
    parser.set_field_boost(f.filename, 3.0);
    parser.set_field_fuzzy(f.filename, false, 1, true);
    parser.set_field_fuzzy(f.body, false, 1, true);

    let (query, _errors) = parser.parse_query_lenient(query_str);

    let top = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;

    let mut hits = Vec::with_capacity(top.len());
    for (score, addr) in top {
        let doc: TantivyDocument = searcher.doc(addr)?;
        let path = doc
            .get_first(f.path)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let body = doc
            .get_first(f.body)
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let snippet = make_snippet(body, query_str, 80);
        hits.push(Hit {
            path,
            score,
            snippet,
        });
    }

    Ok(hits)
}

fn words_with_offsets(s: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices() {
        if c.is_alphanumeric() {
            start.get_or_insert(i);
        } else if let Some(st) = start.take() {
            out.push((st, &s[st..i]));
        }
    }
    if let Some(st) = start {
        out.push((st, &s[st..]));
    }
    out
}

/// Text around the first body word matching a query term (exact, else within 1 edit).
fn make_snippet(body: &str, query: &str, radius: usize) -> String {
    let terms: Vec<String> = words_with_offsets(query)
        .into_iter()
        .map(|(_, w)| w.to_lowercase())
        .collect();

    let words: Vec<(usize, String)> = words_with_offsets(body)
        .into_iter()
        .map(|(i, w)| (i, w.to_lowercase()))
        .collect();

    let pos = words
        .iter()
        .find(|(_, w)| terms.iter().any(|t| t == w))
        .or_else(|| {
            words
                .iter()
                .find(|(_, w)| terms.iter().any(|t| damerau_levenshtein(t, w) <= 1))
        })
        .map(|(i, w)| (*i, w.len()));

    let (start, end) = match pos {
        Some((i, len)) => (i.saturating_sub(radius), (i + len + radius).min(body.len())),
        None => (0, radius * 2), // filename-only match: show the start of the doc
    };

    // Snap to valid UTF-8 boundaries
    let mut start = start.min(body.len());
    while !body.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = end.min(body.len());
    while !body.is_char_boundary(end) {
        end += 1;
    }

    let text = body[start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let pre = if start > 0 { "…" } else { "" };
    let post = if end < body.len() { "…" } else { "" };
    format!("{pre}{text}{post}")
}
