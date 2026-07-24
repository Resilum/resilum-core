use serde::{Deserialize, Serialize};

use leviculum_std::api::Destination;

use super::{APP_NAME, ASPECT};

pub fn name_hash() -> Vec<u8> {
    Destination::compute_name_hash(APP_NAME, ASPECT).to_vec()
}

const VERSION: &str = match option_env!("RESILUM_VERSION") {
    Some(v) => v,
    None => "0.0.0",
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Advert {
    pub v: String,
    /// Hex-encoded rngit Repositories Destination hash — the address peers
    /// dial to reach `rns://<rngit>/mirrors/<repo>`.
    pub rngit: String,
    pub repos: Vec<String>,
}

impl Advert {
    pub fn new(rngit_dest_hex: String, repos: Vec<String>) -> Self {
        Self {
            v: VERSION.to_owned(),
            rngit: rngit_dest_hex,
            repos,
        }
    }
}

pub fn pack(advert: &Advert) -> Vec<u8> {
    serde_json::to_vec(advert).expect("advert serializes")
}

pub fn parse(raw: &[u8]) -> Option<Advert> {
    let advert: Advert = serde_json::from_slice(raw).ok()?;
    (!advert.rngit.is_empty()
        && !advert.repos.is_empty()
        && advert.rngit.len() == 32
        && advert.rngit.bytes().all(|b| b.is_ascii_hexdigit()))
    .then_some(advert)
}
