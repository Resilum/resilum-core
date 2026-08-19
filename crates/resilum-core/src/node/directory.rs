use super::Node;
use crate::discovery::Service;

impl Node {
    #[must_use]
    pub fn discovered(&self, service: Service) -> Vec<[u8; 16]> {
        self.directories
            .get(&service)
            .map_or_else(Vec::new, |directory| directory.discovered())
    }

    pub fn advertise(&self, service: Service, address: [u8; 16]) {
        if let Some(directory) = self.directories.get(&service) {
            directory.announce(address);
        }
    }

    pub fn stop_advertising(&self, service: Service) {
        if let Some(directory) = self.directories.get(&service) {
            directory.withdraw();
        }
    }
}
