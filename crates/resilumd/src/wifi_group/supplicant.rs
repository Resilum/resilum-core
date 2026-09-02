use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::time::Duration;

use resilum_core::WifiGroup;

use super::{Lowering, RaisesAGroup};

const WHERE_IT_LISTENS: &str = "/run/wpa_supplicant";
const ANSWERS_WITHIN: Duration = Duration::from_secs(5);
const THE_2GHZ_CHANNEL_WE_HOST_ON: &str = "2437";

pub struct WpaSupplicant;

pub fn if_it_is_running() -> Option<WpaSupplicant> {
    Path::new(WHERE_IT_LISTENS)
        .read_dir()
        .ok()?
        .next()
        .map(|_| WpaSupplicant)
}

impl RaisesAGroup for WpaSupplicant {
    fn raise(&self, group: &WifiGroup, interface: &str) -> Result<Lowering, String> {
        let control = Control::to(interface)?;
        let network = control.asked("ADD_NETWORK")?.trim().to_owned();
        for (key, value) in describing(group) {
            control.told(&format!("SET_NETWORK {network} {key} {value}"))?;
        }
        control.told(&format!("SELECT_NETWORK {network}"))?;
        let interface = interface.to_owned();
        Ok(Box::new(move || forget(&interface, &network)))
    }

    fn addresses_the_interface_itself(&self) -> bool {
        false
    }
}

fn describing(group: &WifiGroup) -> Vec<(&'static str, String)> {
    vec![
        ("ssid", format!("\"{}\"", group.ssid)),
        ("psk", format!("\"{}\"", group.passphrase)),
        ("mode", String::from("2")),
        ("frequency", String::from(THE_2GHZ_CHANNEL_WE_HOST_ON)),
        ("key_mgmt", String::from("WPA-PSK")),
        ("proto", String::from("RSN")),
        ("pairwise", String::from("CCMP")),
        ("group", String::from("CCMP")),
    ]
}

fn forget(interface: &str, network: &str) {
    let Ok(control) = Control::to(interface) else {
        return;
    };
    for order in [
        format!("DISABLE_NETWORK {network}"),
        format!("REMOVE_NETWORK {network}"),
    ] {
        if let Err(error) = control.told(&order) {
            tracing::warn!(%error, "wpa_supplicant kept the group network");
        }
    }
}

struct Control {
    socket: UnixDatagram,
    ours: PathBuf,
}

impl Control {
    fn to(interface: &str) -> Result<Self, String> {
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

    fn asked(&self, order: &str) -> Result<String, String> {
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

    fn told(&self, order: &str) -> Result<(), String> {
        self.asked(order).map(drop)
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ours);
    }
}
