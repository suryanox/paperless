use anyhow::Result;

use crate::index;

pub fn run(query: String, limit: usize) -> Result<()> {
    let hits = index::search(&query, limit)?;

    if hits.is_empty() {
        println!("No results for {:?}", query);
        return Ok(());
    }

    for (i, hit) in hits.iter().enumerate() {
        println!("{}. {}  ({:.2})", i + 1, hit.path, hit.score);
        if !hit.snippet.is_empty() {
            println!("   {}", hit.snippet);
        }
    }

    Ok(())
}
