use std::path::{Path, PathBuf};

use super::Entry;

const SERVED: &str = "served";
const WANTED: &str = "wanted";

pub struct SharedWithRngit {
    state: PathBuf,
}

impl SharedWithRngit {
    #[must_use]
    pub fn beside(destination_file: &Path) -> Self {
        Self {
            state: destination_file
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf(),
        }
    }

    #[must_use]
    pub fn of_these_we_serve(&self, wanted: &[String]) -> Vec<String> {
        let served = read_lines(&self.state.join(SERVED));
        wanted
            .iter()
            .filter(|repo| served.iter().any(|line| line == *repo))
            .cloned()
            .collect()
    }

    pub fn ask_for_what_we_lack(&self, wanted: &[String], known: &[Entry]) {
        let served = read_lines(&self.state.join(SERVED));
        let asking = whom_to_ask(wanted, &served, known);
        if asking.is_empty() {
            return;
        }
        if let Err(error) =
            resilum_store::write_text(&self.state.join(WANTED), &(asking.join("\n") + "\n"))
        {
            tracing::debug!(%error, "could not leave rngit a list of mirrors to fetch");
        }
    }
}

pub(super) fn read_rngit_destination(path: &Path) -> Option<String> {
    let raw = resilum_store::read_text(path).ok()?;
    let trimmed = raw.trim();
    (trimmed.len() == 32 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| trimmed.to_owned())
}

fn whom_to_ask(wanted: &[String], served: &[String], known: &[Entry]) -> Vec<String> {
    wanted
        .iter()
        .filter(|repo| !served.iter().any(|had| had == *repo))
        .filter_map(|repo| {
            let from = known
                .iter()
                .find(|entry| entry.repos.iter().any(|theirs| theirs == repo))?;
            Some(format!("{repo}\trns://{}/mirrors/{repo}", from.rngit))
        })
        .collect()
}

fn read_lines(path: &Path) -> Vec<String> {
    resilum_store::read_text(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests;
