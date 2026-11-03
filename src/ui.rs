use std::path::Path;
use std::time::Duration;

use console::style;
use directories::UserDirs;
use indicatif::{HumanBytes, HumanDuration, ProgressBar, ProgressStyle};

use crate::index::Hit;
use crate::pdf::ExtractStats;

pub fn shorten_path(p: &Path) -> String {
    if let Some(home) = UserDirs::new().map(|u| u.home_dir().to_path_buf()) {
        if let Ok(rest) = p.strip_prefix(&home) {
            return if rest.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", rest.display())
            };
        }
    }
    p.display().to_string()
}

fn highlight(text: &str, ranges: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for &(s, e) in ranges {
        out.push_str(&text[last..s]);
        out.push_str(&style(&text[s..e]).bold().yellow().to_string());
        last = e;
    }
    out.push_str(&text[last..]);
    out
}

pub fn print_hit(rank: usize, hit: &Hit) {
    let path = Path::new(&hit.path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path.parent().map(shorten_path).unwrap_or_default();

    println!("{}  {}", style(rank).dim(), style(name).bold());
    println!(
        "   {}",
        style(format!("{}/", dir.trim_end_matches('/'))).dim()
    );
    if !hit.snippet.is_empty() {
        println!("   {}", highlight(&hit.snippet, &hit.matches));
    }
    println!();
}

pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.cyan} {msg}").unwrap());
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn print_init_summary(stats: &ExtractStats, bytes: u64, elapsed: Duration) {
    println!(
        "{} Indexed {} PDFs ({}) in {}",
        style("✓").green().bold(),
        style(stats.succeeded).bold(),
        HumanBytes(bytes),
        HumanDuration(elapsed)
    );
    if stats.failed > 0 {
        println!(
            "  {} {} failed to extract",
            style("!").red().bold(),
            stats.failed
        );
    }
    if stats.empty > 0 {
        println!(
            "  {} {} have no text (likely scans)",
            style("!").yellow().bold(),
            stats.empty
        );
    }
}

pub fn print_update_summary(add: usize, upd: usize, rem: usize, unchanged: usize, failed: usize) {
    println!(
        "{} {}  {}  {}  {}",
        style("✓").green().bold(),
        style(format!("+{add} added")).green(),
        style(format!("~{upd} updated")).yellow(),
        style(format!("-{rem} removed")).red(),
        style(format!("{unchanged} unchanged")).dim(),
    );
    if failed > 0 {
        println!("  {} {} failed to extract", style("!").red().bold(), failed);
    }
}

pub fn up_to_date(unchanged: usize) {
    println!(
        "{} Up to date {}",
        style("✓").green().bold(),
        style(format!("({unchanged} unchanged)")).dim()
    );
}
