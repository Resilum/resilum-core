//! Checks that need more than a shell one-liner, kept in the project's own
//! language rather than embedded in checker.sh.

mod density;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("comment-density") => match args.get(1).and_then(|p| p.parse().ok()) {
            Some(limit) => comment_density(limit, &args[2..]),
            None => usage(),
        },
        Some("file-length") => match args.get(1).and_then(|p| p.parse().ok()) {
            Some(limit) => file_length(limit, &args[2..]),
            None => usage(),
        },
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: xtask comment-density|file-length <max> <dir>...");
    ExitCode::FAILURE
}

fn file_length(limit: usize, roots: &[String]) -> ExitCode {
    let mut over = Vec::new();
    for root in roots {
        for path in rust_files(Path::new(root)) {
            let Ok(source) = resilum_store::read_text(&path) else {
                continue;
            };
            let lines = source.lines().count();
            if lines > limit {
                over.push((lines, path));
            }
        }
    }
    if over.is_empty() {
        return ExitCode::SUCCESS;
    }
    over.sort_unstable_by_key(|(lines, _)| std::cmp::Reverse(*lines));
    for (lines, path) in over {
        println!("  {lines}\t{}", path.display());
    }
    println!("  ✗ file(s) over {limit} lines; split into a directory module");
    ExitCode::FAILURE
}

fn comment_density(limit: usize, roots: &[String]) -> ExitCode {
    let mut over = Vec::new();
    for root in roots {
        for path in rust_files(Path::new(root)) {
            let Ok(source) = resilum_store::read_text(&path) else {
                continue;
            };
            let filled = density::filled_lines(&source);
            if filled == 0 {
                continue;
            }
            let pct = density::comment_lines(&source) * 100 / filled;
            if pct > limit {
                over.push((pct, path));
            }
        }
    }
    if over.is_empty() {
        return ExitCode::SUCCESS;
    }
    over.sort_unstable_by_key(|(pct, _)| std::cmp::Reverse(*pct));
    for (pct, path) in over {
        println!("  {pct}%  {}", path.display());
    }
    println!("  ✗ comment density over {limit}%; cut what the code already says");
    ExitCode::FAILURE
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = resilum_store::list(&dir) else {
            continue;
        };
        for path in entries {
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }
    found
}
