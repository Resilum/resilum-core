use std::fmt::Display;
use std::io::Write as _;

pub fn shown(text: impl Display) {
    let mut out = std::io::stdout().lock();
    if write!(out, "{text}").and_then(|()| out.flush()).is_err() {
        tracing::debug!("whoever asked stopped reading before the answer was out");
    }
}

pub fn shown_as_a_line(text: impl Display) {
    shown(format_args!("{text}\n"));
}

pub fn refused(text: impl Display) {
    let mut out = std::io::stderr().lock();
    if writeln!(out, "{text}").is_err() {
        tracing::debug!("the refusal never reached the terminal");
    }
}
