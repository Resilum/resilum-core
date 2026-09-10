mod grid;
mod paint;
mod plot;
mod render;
mod tables;
mod units;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use resilum_core::status::NodeStatus;

const A_SNAPSHOT_THIS_OLD_IS_NOT_A_LIVE_NODE: Duration = Duration::from_secs(45);
const WHERE_THE_IMAGE_KEEPS_ITS_CONFIG: &str = "/config/resilumd.yaml";

pub struct Asked {
    pub map: bool,
    pub interfaces: bool,
    pub links: bool,
}

pub fn run(argv: &[String]) -> i32 {
    let json_wanted = argv.iter().any(|a| a == "--json");
    if argv.iter().any(|a| a == "--color") {
        owo_colors::set_override(true);
    }
    let everything = argv.iter().any(|a| a == "--all");
    let asked = Asked {
        map: everything || argv.iter().any(|a| a == "--map"),
        interfaces: everything || argv.iter().any(|a| a == "--interfaces"),
        links: everything || argv.iter().any(|a| a == "--links"),
    };
    let config = argv.iter().find(|a| !a.starts_with("--"));
    let path = where_it_lands(config.map(String::as_str));
    let Ok(raw) = std::fs::read_to_string(&path) else {
        eprintln!("no status at {}", path.display());
        return 1;
    };
    if json_wanted {
        println!("{raw}");
    }
    let status: NodeStatus = match serde_json::from_str(&raw) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("{}: {error}", path.display());
            return 1;
        }
    };
    let age = age_of(&path);
    if !json_wanted {
        print!("{}", render::all_of_it(&status, age, &asked));
    }
    match age {
        Some(age) if age <= A_SNAPSHOT_THIS_OLD_IS_NOT_A_LIVE_NODE && status.running => 0,
        _ => 1,
    }
}

fn where_it_lands(config: Option<&str>) -> PathBuf {
    let env = std::env::var("RESILUM_STATUS_FILE").ok();
    let storage = config
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(WHERE_THE_IMAGE_KEEPS_ITS_CONFIG)).filter(|p| p.is_file()))
        .and_then(|path| crate::config::load(&path).ok())
        .and_then(|cfg| cfg.storage_path);
    super::file_path(storage.as_deref(), env)
}

fn age_of(path: &Path) -> Option<Duration> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    SystemTime::now().duration_since(modified).ok()
}

#[cfg(test)]
mod tests;
