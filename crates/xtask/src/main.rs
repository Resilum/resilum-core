//! Checks that need more than a shell one-liner, kept in the project's own
//! language rather than embedded in checker.sh.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod density;
mod modules;
mod out;

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
        Some("modules-after-imports") => match args.get(1).map(String::as_str) {
            Some("--check") => modules_after_imports(false, &args[2..]),
            Some(_) => modules_after_imports(true, &args[1..]),
            None => usage(),
        },
        _ => usage(),
    }
}

const USAGE: &str = "usage: xtask comment-density|file-length <max> <dir>...
       xtask modules-after-imports [--check] <dir>...";

fn usage() -> ExitCode {
    out::refused(USAGE);
    ExitCode::FAILURE
}

fn modules_after_imports(put_them_there: bool, roots: &[String]) -> ExitCode {
    let mut out_of_order = Vec::new();
    for root in roots {
        for path in rust_files(Path::new(root)) {
            let Ok(source) = resilum_store::read_text(&path) else {
                continue;
            };
            let Some(put) = modules::put_modules_after_imports(&source) else {
                continue;
            };
            if put_them_there && resilum_store::write_text(&path, &put).is_ok() {
                continue;
            }
            out_of_order.push(path);
        }
    }
    let named = out_of_order
        .into_iter()
        .map(|path| format!("  {}", path.display()))
        .collect();
    complain(
        named,
        "module declarations above the imports; run ./checker.sh",
    )
}

fn complain(found: Vec<String>, why: &str) -> ExitCode {
    if found.is_empty() {
        return ExitCode::SUCCESS;
    }
    out::shown(format_args!("{}\n  ✗ {why}", found.join("\n")));
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
    over.sort_unstable_by_key(|(lines, _)| std::cmp::Reverse(*lines));
    let named = over
        .into_iter()
        .map(|(lines, path)| format!("  {lines}\t{}", path.display()))
        .collect();
    complain(
        named,
        &format!("file(s) over {limit} lines; split into a directory module"),
    )
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
    over.sort_unstable_by_key(|(pct, _)| std::cmp::Reverse(*pct));
    let named = over
        .into_iter()
        .map(|(pct, path)| format!("  {pct}%  {}", path.display()))
        .collect();
    complain(
        named,
        &format!("comment density over {limit}%; cut what the code already says"),
    )
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
