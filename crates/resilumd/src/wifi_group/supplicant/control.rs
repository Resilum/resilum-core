use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::WHERE_IT_LISTENS;

const ANSWERS_WITHIN: Duration = Duration::from_secs(5);

pub(super) struct Control {
    socket: UnixDatagram,
    ours: PathBuf,
}

impl Control {
    pub(super) fn to(interface: &str) -> Result<Self, String> {
        let ours =
            std::env::temp_dir().join(format!("resilum-wpa-{}-{interface}", std::process::id()));
        let _ = std::fs::remove_file(&ours);
        let socket = UnixDatagram::bind(&ours).map_err(|e| format!("no control socket: {e}"))?;
        socket
            .connect(Path::new(WHERE_IT_LISTENS).join(interface))
            .map_err(|e| format!("wpa_supplicant holds no {interface}: {e}"))?;
        socket
            .set_read_timeout(Some(ANSWERS_WITHIN))
            .map_err(|e| format!("no timeout on the control socket: {e}"))?;
        Ok(Self { socket, ours })
    }

    pub(super) fn asked(&self, order: &str) -> Result<String, String> {
        self.socket
            .send(order.as_bytes())
            .map_err(|e| format!("{order} never went out: {e}"))?;
        let mut answer = [0u8; 512];
        let len = self
            .socket
            .recv(&mut answer)
            .map_err(|e| format!("{order} went unanswered: {e}"))?;
        let said = String::from_utf8_lossy(&answer[..len]).into_owned();
        if said.starts_with("FAIL") {
            return Err(format!("{order} was refused"));
        }
        Ok(said)
    }

    pub(super) fn told(&self, order: &str) -> Result<(), String> {
        self.asked(order).map(drop)
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ours);
    }
}
