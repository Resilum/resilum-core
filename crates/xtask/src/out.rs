use std::fmt::Display;
use std::io::Write as _;

pub fn shown(text: impl Display) {
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{text}").is_err() {
        std::process::exit(1);
    }
}

pub fn refused(text: impl Display) {
    let mut out = std::io::stderr().lock();
    if writeln!(out, "{text}").is_err() {
        std::process::exit(1);
    }
}
