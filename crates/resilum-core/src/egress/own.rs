use std::collections::HashSet;

use tokio::net::TcpStream;

use crate::config::EgressListen;
use crate::egress::Candidate;

#[derive(Clone)]
pub struct OwnExit {
    pub service: String,
    pub dest_hash: Vec<u8>,
    pub target: Option<String>,
    pub exit_country: String,
}

#[derive(Default, Clone)]
pub struct OwnExits {
    exits: Vec<OwnExit>,
}

impl OwnExits {
    pub fn of(configured: &[EgressListen], dest_hash_of: impl Fn(&str) -> Vec<u8>) -> Self {
        let exits = configured
            .iter()
            .map(|exit| OwnExit {
                service: exit.service.clone(),
                dest_hash: dest_hash_of(&exit.service),
                target: exit.target.clone(),
                exit_country: exit.exit_country.clone(),
            })
            .collect();
        Self { exits }
    }

    pub fn is_ours(&self, candidate: &Candidate) -> bool {
        self.exits
            .iter()
            .any(|exit| exit.dest_hash == candidate.dest_hash)
    }

    pub fn target_of(&self, candidate: &Candidate) -> Option<&str> {
        self.exits
            .iter()
            .find(|exit| exit.dest_hash == candidate.dest_hash)
            .and_then(|exit| exit.target.as_deref())
    }

    pub fn hashes_serving(&self, service: &str) -> HashSet<Vec<u8>> {
        self.exits
            .iter()
            .filter(|exit| exit.service == service)
            .map(|exit| exit.dest_hash.clone())
            .collect()
    }

    pub fn with_a_socket_this_node_can_dial<'a>(
        &'a self,
        wanted: &'a [String],
    ) -> impl Iterator<Item = &'a OwnExit> {
        self.exits
            .iter()
            .filter(|exit| exit.target.is_some() && wanted.contains(&exit.service))
    }
}

pub async fn session(target: &str, mut tcp: TcpStream) -> std::io::Result<()> {
    let mut exit = TcpStream::connect(target).await?;
    tokio::io::copy_bidirectional(&mut tcp, &mut exit).await?;
    Ok(())
}

#[cfg(test)]
mod tests;
