use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::Config;

const A_NAME_ANSWERS_WITHIN: Duration = Duration::from_secs(2);

#[must_use]
pub(crate) fn only_those_that_resolve(config: &Config) -> Config {
    let answered = those_that_answer(
        config
            .bootstrap
            .iter()
            .chain(&config.bootstrap_only)
            .cloned(),
    );
    let mut trimmed = config.clone();
    trimmed.bootstrap = kept(&config.bootstrap, |name| answered.contains(name));
    trimmed.bootstrap_only = kept(&config.bootstrap_only, |name| answered.contains(name));
    trimmed
}

fn kept(names: &[String], answers: impl Fn(&str) -> bool) -> Vec<String> {
    names
        .iter()
        .filter(|name| {
            let heard = answers(name);
            if !heard {
                tracing::warn!(%name, "left an anchor out: its name does not resolve here");
            }
            heard
        })
        .cloned()
        .collect()
}

fn those_that_answer(names: impl Iterator<Item = String>) -> HashSet<String> {
    let asked: Vec<(String, mpsc::Receiver<bool>)> = names
        .map(|name| {
            let (tell, heard) = mpsc::channel();
            let asking = name.clone();
            std::thread::spawn(move || {
                let _ = tell.send(resolves(&asking));
            });
            (name, heard)
        })
        .collect();
    let give_up_at = Instant::now() + A_NAME_ANSWERS_WITHIN;
    asked
        .into_iter()
        .filter_map(|(name, heard)| {
            let left = give_up_at.saturating_duration_since(Instant::now());
            heard.recv_timeout(left).unwrap_or(false).then_some(name)
        })
        .collect()
}

fn resolves(name: &str) -> bool {
    name.to_socket_addrs()
        .map(|mut found| found.next().is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
