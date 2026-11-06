use std::time::Instant;

use anyhow::Result;

use crate::{index, ui};

pub fn run(query: String, limit: usize, json: bool) -> Result<()> {
    let start = Instant::now();
    let hits = index::search(&query, limit)?;
    let elapsed = start.elapsed();

    if json {
        println!("{}", serde_json::to_string_pretty(&hits)?);
        return Ok(());
    }

    let width = ui::term_width();
    ui::print_search_header(&query, hits.len(), elapsed, width);

    if hits.is_empty() {
        ui::no_results();
        return Ok(());
    }

    for (i, hit) in hits.iter().enumerate() {
        ui::print_hit(i + 1, hit, width);
    }
    Ok(())
}
