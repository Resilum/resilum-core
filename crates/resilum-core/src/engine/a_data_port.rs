use std::net::{Ipv6Addr, SocketAddrV6, UdpSocket};

pub(crate) const WHAT_RETICULUM_EXPECTS: u16 = 42671;

#[must_use]
pub(crate) fn nobody_else_holds(preferred: u16) -> u16 {
    if can_be_bound(preferred) {
        return preferred;
    }
    let taken_instead = whatever_the_kernel_hands_out();
    tracing::info!(
        preferred,
        chose = taken_instead,
        "another Reticulum node holds the usual data port; peers learn ours from discovery"
    );
    taken_instead
}

fn can_be_bound(port: u16) -> bool {
    UdpSocket::bind(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0)).is_ok()
}

fn whatever_the_kernel_hands_out() -> u16 {
    UdpSocket::bind(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0))
        .and_then(|socket| socket.local_addr())
        .map_or(WHAT_RETICULUM_EXPECTS, |addr| addr.port())
}

#[cfg(test)]
mod tests;
