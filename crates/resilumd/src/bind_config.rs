//! Splice managed listen-address regions into Yggdrasil and Reticulum configs
//! from env-provided values.

mod parsing;
mod regions;
mod render;

use std::path::Path;

use parsing::{RNS_ENV, YGG_ENV};
use render::{render_rns, render_ygg};

const RNS_CONFIG_DIR_ENV: &str = "RNS_CONFIG_DIR";

pub fn run(ygg_path: Option<&str>, rns_path: Option<&str>) -> i32 {
    let ygg = ygg_path.unwrap_or("/config/yggdrasil.conf");
    let rns = rns_path.map(String::from).unwrap_or_else(default_rns_path);
    let mut rc = 0;
    if let Err(code) = render_file(Path::new(ygg), "ygg-public-listen", YGG_ENV, render_ygg) {
        rc = code;
    }
    if let Err(code) = render_file(Path::new(&rns), "rns-public-listen", RNS_ENV, render_rns) {
        rc = code;
    }
    rc
}

fn default_rns_path() -> String {
    let dir = std::env::var(RNS_CONFIG_DIR_ENV).unwrap_or_else(|_| "/config/reticulum".into());
    format!("{dir}/config")
}

fn render_file(
    path: &Path,
    tag: &str,
    env: &str,
    renderer: fn(&str, &str) -> Result<Option<String>, String>,
) -> Result<(), i32> {
    let Ok(value) = std::env::var(env) else {
        return Ok(());
    };
    let original = resilum_store::read_text(path).map_err(|e| {
        tracing::error!(path = %path.display(), error = %e, "read failed");
        1
    })?;
    let updated = renderer(&original, &value).map_err(|e| {
        tracing::error!(env, error = %e, "invalid value");
        2
    })?;
    let Some(text) = updated else {
        tracing::warn!(
            env,
            path = %path.display(),
            "no resilum:managed region; value ignored"
        );
        return Ok(());
    };
    resilum_store::write_text(path, &text).map_err(|e| {
        tracing::error!(path = %path.display(), error = %e, "write failed");
        1
    })?;
    tracing::info!(tag, path = %path.display(), "rendered");
    Ok(())
}

#[cfg(test)]
mod tests;
