//! Colour only where a terminal is watching: `owo-colors` checks the stream,
//! `NO_COLOR` and `FORCE_COLOR`, so a redirect into a file stays plain.

use owo_colors::{OwoColorize, Stream::Stdout};

pub fn a_mark(text: &str, service: Option<&str>) -> String {
    bold(&transport_ink(text, service.unwrap_or("other")))
}

fn ink(service: &str) -> (u8, u8, u8) {
    match service.split(['_', '/']).next().unwrap_or(service) {
        "tor" => (0x7D, 0x46, 0x98),
        "i2p" => (0x52, 0x6B, 0xCE),
        "yggdrasil" => (0x00, 0x99, 0x99),
        "iroh" => (0x7C, 0x7C, 0xFF),
        "covert" => (0xFF, 0x8F, 0x00),
        "ble" => (0x00, 0x82, 0xFC),
        "wifi" => (0xFF, 0xC1, 0x07),
        "direct" => (0x8B, 0xC3, 0x4A),
        _ => (0x9E, 0x9E, 0x9E),
    }
}

pub fn bold(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.bold()).to_string()
}

pub fn dimmed(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.dimmed()).to_string()
}

pub fn green(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.green()).to_string()
}

pub fn red(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.red()).to_string()
}

pub fn yellow(text: &str) -> String {
    text.if_supports_color(Stdout, |t| t.yellow()).to_string()
}

pub fn transport(text: &str) -> String {
    transport_ink(text, text)
}

fn transport_ink(text: &str, service: &str) -> String {
    let (r, g, b) = ink(service);
    text.if_supports_color(Stdout, |t| t.truecolor(r, g, b))
        .to_string()
}
