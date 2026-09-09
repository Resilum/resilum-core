use std::net::Ipv4Addr;
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use super::address::{as_wide_as_this_libc_wants, named};

const AN_ADDRESS_SETTLES_WITHIN: Duration = Duration::from_secs(5);
const BETWEEN_LOOKS: Duration = Duration::from_millis(100);

pub fn settles_on(interface: &str, owner: Ipv4Addr) -> Result<(), String> {
    took_hold(owner, || held_by(interface)).map_err(|instead| {
        format!(
            "{interface} never took {owner}; it holds {}",
            instead.map_or_else(|| String::from("no address at all"), |a| a.to_string())
        )
    })
}

fn took_hold(
    owner: Ipv4Addr,
    mut look: impl FnMut() -> Option<Ipv4Addr>,
) -> Result<(), Option<Ipv4Addr>> {
    let give_up_at = Instant::now() + AN_ADDRESS_SETTLES_WITHIN;
    loop {
        let held = look();
        if held == Some(owner) {
            return Ok(());
        }
        if Instant::now() >= give_up_at {
            return Err(held);
        }
        std::thread::sleep(BETWEEN_LOOKS);
    }
}

fn held_by(interface: &str) -> Option<Ipv4Addr> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )
    .ok()?;
    let mut request = named(interface).ok()?;
    if unsafe {
        libc::ioctl(
            socket.as_raw_fd(),
            as_wide_as_this_libc_wants(libc::SIOCGIFADDR),
            &raw mut request,
        )
    } < 0
    {
        return None;
    }
    let inet: libc::sockaddr_in = unsafe {
        std::mem::transmute::<libc::sockaddr, libc::sockaddr_in>(request.ifr_ifru.ifru_addr)
    };
    Some(Ipv4Addr::from(inet.sin_addr.s_addr.to_ne_bytes()))
}

#[cfg(test)]
mod tests;
