# paperless

A command line tool that indexes the PDFs on your machine and lets you search them, including typos.

## What it does

`paperless init` walks your folders, pulls the text out of every PDF, and builds a search index. After that, `paperless search` finds PDFs by filename or by what's inside them, and shows the file path with a snippet of text around the match.

## How it's built

Written in Rust.

- `clap` for the CLI
- `walkdir` to find PDFs
- `pdf-extract` to read the text
- `rayon` to extract in parallel
- `tantivy` for the search index and fuzzy matching
- `indicatif` and `console` for progress bars and colors

## How it stores the index

Everything lives in your OS data directory, for example `~/.local/share/paperless` on Linux and `~/Library/Application Support/paperless` on macOS.

- `index/` is the tantivy index. Each PDF is one document with these fields: path, filename, body text, size and modified time.
- `meta.json` stores the folders you indexed, so `update` knows what to rescan.

`update` compares each file's size and modified time against the index. It only re-reads new or changed files and removes entries for deleted ones.

## Install

```bash
cargo build --release
```

The binary ends up in `target/release/paperless`.

## Commands

```bash
paperless init                         # index your home directory
paperless init --path ~/Documents      # index specific folders (repeatable)
paperless init --force                 # delete the index and rebuild it

paperless update                       # re-scan, only process changes
paperless update --path ~/Downloads    # scan a different folder

paperless search tax return 2023       # search, quotes are optional
paperless search invoice --limit 20    # change the number of results
paperless search invoice --json        # JSON output for scripts

paperless --help
```