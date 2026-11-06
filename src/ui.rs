use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use console::{Term, measure_text_width, style, truncate_str};
use directories::UserDirs;
use indicatif::{HumanBytes, HumanDuration, ProgressBar, ProgressStyle};

use crate::index::{self, Hit};
use crate::pdf::ExtractStats;

pub fn term_width() -> usize {
    (Term::stdout().size().1 as usize).clamp(60, 100)
}

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

pub fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn ago(mtime: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs = now.saturating_sub(mtime);
    if secs < 60 {
        "just now".to_string()
    } else {
        format!("{} ago", HumanDuration(Duration::from_secs(secs)))
    }
}

fn wrap_ranges(text: &str, width: usize) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let (mut ls, mut le, mut lw) = (0, 0, 0);
    let mut pos = 0;

    for word in text.split(' ') {
        let (start, end) = (pos, pos + word.len());
        let w = measure_text_width(word);
        pos = end + 1;

        if lw == 0 {
            (ls, le, lw) = (start, end, w);
        } else if lw + 1 + w <= width {
            le = end;
            lw += 1 + w;
        } else {
            lines.push((ls, le));
            (ls, le, lw) = (start, end, w);
        }
    }
    if lw > 0 {
        lines.push((ls, le));
    }
    lines
}

fn highlight_line(text: &str, (ls, le): (usize, usize), ranges: &[(usize, usize)]) -> String {
    let mut out = String::new();
    let mut last = ls;
    for &(s, e) in ranges.iter().filter(|(s, e)| *s >= ls && *e <= le) {
        out.push_str(&text[last..s]);
        out.push_str(&style(&text[s..e]).bold().yellow().to_string());
        last = e;
    }
    out.push_str(&text[last..le]);
    out
}

pub fn print_search_header(query: &str, n: usize, elapsed: Duration, width: usize) {
    println!(
        "{} {}",
        style("paperless").bold().cyan(),
        style(format!(
            "· {} result{} for \"{}\" ({}ms)",
            n,
            if n == 1 { "" } else { "s" },
            query,
            elapsed.as_millis()
        ))
        .dim()
    );
    println!("{}\n", style("─".repeat(width)).dim());
}

pub fn no_results() {
    println!("  {}", style("Nothing matched.").bold());
    println!(
        "  {}",
        style("Try fewer words, or run `paperless update` if the file is new.").dim()
    );
}

pub fn print_hit(rank: usize, hit: &Hit, width: usize) {
    const INDENT: usize = 4;

    let path = Path::new(&hit.path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path.parent().map(shorten_path).unwrap_or_default();
    let meta = format!("{} · {}", HumanBytes(hit.size), ago(hit.mtime));

    let meta_w = measure_text_width(&meta);
    let max_name = width.saturating_sub(INDENT + meta_w + 2).max(10);
    let name = truncate_str(&name, max_name, "…");
    let pad = width.saturating_sub(INDENT + measure_text_width(&name) + meta_w);

    println!(
        "{}  {}{}{}",
        style(format!("{rank:>2}")).dim(),
        style(&name).bold().cyan(),
        " ".repeat(pad),
        style(meta).dim()
    );
    println!("    {}", style(dir).blue().dim());

    if !hit.snippet.is_empty() {
        println!();
        for line in wrap_ranges(&hit.snippet, width - INDENT) {
            println!("    {}", highlight_line(&hit.snippet, line, &hit.matches));
        }
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

pub fn progress_bar(total: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{prefix:.bold} {bar:32.cyan/238} {pos}/{len}  {per_sec}  eta {eta}",
        )
        .unwrap()
        .progress_chars("━╸─"),
    );
    pb.set_prefix(prefix.to_string());
    pb
}

pub fn done(msg: &str) {
    println!("{} {}", style("✓").green().bold(), msg);
}

fn row(label: &str, value: String, note: &str) {
    println!(
        "  {:<14}{}  {}",
        style(label).dim(),
        value,
        style(note).dim()
    );
}

pub fn print_init_summary(stats: &ExtractStats, bytes: u64, elapsed: Duration) {
    println!();
    println!("{}", style("Index ready").green().bold());
    row(
        "Indexed",
        style(fmt_num(stats.succeeded)).bold().to_string(),
        &format!("PDFs ({})", HumanBytes(bytes)),
    );
    if stats.empty > 0 {
        row(
            "No text",
            style(fmt_num(stats.empty)).yellow().to_string(),
            "likely scans",
        );
    }
    if stats.failed > 0 {
        row(
            "Failed",
            style(fmt_num(stats.failed)).red().to_string(),
            "could not be read",
        );
    }
    row("Time", HumanDuration(elapsed).to_string(), "");
    if let Ok(dir) = index::index_dir() {
        row("Location", shorten_path(&dir), "");
    }
}

pub fn print_update_summary(add: usize, upd: usize, rem: usize, unchanged: usize, failed: usize) {
    println!(
        "{} {}  {}  {}  {}",
        style("✓").green().bold(),
        style(format!("+{add} added")).green(),
        style(format!("~{upd} updated")).yellow(),
        style(format!("-{rem} removed")).red(),
        style(format!("{} unchanged", fmt_num(unchanged))).dim(),
    );
    if failed > 0 {
        println!("  {} {} failed to extract", style("!").red().bold(), failed);
    }
}

pub fn up_to_date(unchanged: usize) {
    println!(
        "{} Up to date {}",
        style("✓").green().bold(),
        style(format!("({} unchanged)", fmt_num(unchanged))).dim()
    );
}
