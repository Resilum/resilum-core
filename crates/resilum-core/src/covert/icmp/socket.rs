use std::io;

use socket2::{Domain, Protocol, Socket, Type};

#[derive(Clone, Copy)]
pub enum HowTheKernelHandsItOver {
    StartingAtTheIcmpHeader,
    BehindTheIpv4Header,
}

impl HowTheKernelHandsItOver {
    #[must_use]
    pub fn icmp_within(self, arrived: &[u8]) -> &[u8] {
        match self {
            Self::StartingAtTheIcmpHeader => arrived,
            Self::BehindTheIpv4Header => {
                let words = usize::from(arrived.first().copied().unwrap_or(0) & 0x0f);
                arrived.get(words * 4..).unwrap_or_default()
            }
        }
    }
}

pub fn whichever_this_host_allows(
    domain: Domain,
    proto: Protocol,
    v6: bool,
) -> io::Result<(Socket, HowTheKernelHandsItOver)> {
    let unprivileged = match Socket::new(domain, Type::DGRAM, Some(proto)) {
        Ok(sock) => return Ok((sock, HowTheKernelHandsItOver::StartingAtTheIcmpHeader)),
        Err(refused) if refused.kind() == io::ErrorKind::PermissionDenied => refused,
        Err(other) => return Err(other),
    };
    let sock = Socket::new(domain, Type::RAW, Some(proto)).map_err(|raw| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "no icmp socket: unprivileged refused ({unprivileged}), raw refused ({raw}). \
                 Widen net.ipv4.ping_group_range or grant CAP_NET_RAW"
            ),
        )
    })?;
    Ok((
        sock,
        if v6 {
            HowTheKernelHandsItOver::StartingAtTheIcmpHeader
        } else {
            HowTheKernelHandsItOver::BehindTheIpv4Header
        },
    ))
}

#[cfg(test)]
mod tests;
