use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use edge_dhcp::server::{Server, ServerOptions};
use edge_dhcp::{Options, Packet};

const WHERE_CLIENTS_ASK: u16 = 67;
const WHERE_CLIENTS_LISTEN: u16 = 68;
const LEASES_AT_ONCE: usize = 32;
const LOOK_FOR_A_STOP_EVERY: Duration = Duration::from_millis(500);
const LARGEST_MESSAGE: usize = 1024;

pub struct Serving {
    told_to_stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Serving {
    pub fn stop(mut self) {
        self.told_to_stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        self.told_to_stop.store(true, Ordering::Relaxed);
    }
}

pub fn on(interface: &str, owner: Ipv4Addr, seconds: fn() -> u64) -> std::io::Result<Serving> {
    let socket = bound_to(interface)?;
    let told_to_stop = Arc::new(AtomicBool::new(false));
    let stopping = told_to_stop.clone();
    let thread =
        std::thread::spawn(move || answer_until_told_to_stop(&socket, owner, seconds, &stopping));
    Ok(Serving {
        told_to_stop,
        thread: Some(thread),
    })
}

fn bound_to(interface: &str) -> std::io::Result<UdpSocket> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )?;
    socket.set_reuse_address(true)?;
    socket.bind_device(Some(interface.as_bytes()))?;
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(LOOK_FOR_A_STOP_EVERY))?;
    socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, WHERE_CLIENTS_ASK)).into())?;
    Ok(socket.into())
}

fn answer_until_told_to_stop(
    socket: &UdpSocket,
    owner: Ipv4Addr,
    seconds: fn() -> u64,
    told_to_stop: &AtomicBool,
) {
    let mut server: Server<_, LEASES_AT_ONCE> = Server::new(seconds, owner);
    let mut gateway = [owner];
    let ours = ServerOptions::new(owner, Some(&mut gateway));
    let mut asked = [0u8; LARGEST_MESSAGE];
    while !told_to_stop.load(Ordering::Relaxed) {
        let Ok((len, _)) = socket.recv_from(&mut asked) else {
            continue;
        };
        let Ok(request) = Packet::decode(&asked[..len]) else {
            continue;
        };
        let mut options = Options::buf();
        let Some(reply) = server.handle_request(&mut options, &ours, &request) else {
            continue;
        };
        let mut answer = [0u8; LARGEST_MESSAGE];
        let Ok(bytes) = reply.encode(&mut answer) else {
            continue;
        };
        let to = SocketAddrV4::new(Ipv4Addr::BROADCAST, WHERE_CLIENTS_LISTEN);
        if let Err(error) = socket.send_to(bytes, to) {
            tracing::debug!(%error, "a dhcp answer never went out");
        }
    }
}
