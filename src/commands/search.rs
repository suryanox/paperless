use anyhow::Result;
use console::style;

use crate::{index, ui};

pub fn run(query: String, limit: usize, json: bool) -> Result<()> {
    let hits = index::search(&query, limit)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&hits)?);
        return Ok(());
    }

    if hits.is_empty() {
        println!("No results for {}", style(&query).bold());
        return Ok(());
    }

    for (i, hit) in hits.iter().enumerate() {
        ui::print_hit(i + 1, hit);
    }
    Ok(())
}
