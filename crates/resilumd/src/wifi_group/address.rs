use std::net::Ipv4Addr;
use std::os::fd::AsRawFd;

const A_FULL_24: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 0);

const fn as_wide_as_this_libc_wants(request: u64) -> libc::Ioctl {
    request as libc::Ioctl
}

pub fn put_on(interface: &str, owner: Ipv4Addr) -> Result<(), String> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )
    .map_err(|e| format!("no socket to configure {interface}: {e}"))?;
    let fd = socket.as_raw_fd();
    told(
        fd,
        interface,
        as_wide_as_this_libc_wants(libc::SIOCSIFADDR),
        as_sockaddr(owner),
    )?;
    told(
        fd,
        interface,
        as_wide_as_this_libc_wants(libc::SIOCSIFNETMASK),
        as_sockaddr(A_FULL_24),
    )?;
    brought_up(fd, interface)
}

fn told(
    fd: i32,
    interface: &str,
    order: libc::Ioctl,
    address: libc::sockaddr,
) -> Result<(), String> {
    let mut request = named(interface)?;
    request.ifr_ifru.ifru_addr = address;
    if unsafe { libc::ioctl(fd, order, &raw mut request) } < 0 {
        return Err(format!(
            "{interface} refused an address: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn brought_up(fd: i32, interface: &str) -> Result<(), String> {
    let mut request = named(interface)?;
    if unsafe {
        libc::ioctl(
            fd,
            as_wide_as_this_libc_wants(libc::SIOCGIFFLAGS),
            &raw mut request,
        )
    } < 0
    {
        return Err(format!(
            "{interface} would not say how it stands: {}",
            std::io::Error::last_os_error()
        ));
    }
    unsafe {
        request.ifr_ifru.ifru_flags |= (libc::IFF_UP | libc::IFF_RUNNING) as libc::c_short;
    }
    if unsafe {
        libc::ioctl(
            fd,
            as_wide_as_this_libc_wants(libc::SIOCSIFFLAGS),
            &raw mut request,
        )
    } < 0
    {
        return Err(format!(
            "{interface} would not come up: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn named(interface: &str) -> Result<libc::ifreq, String> {
    let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
    let room = request.ifr_name.len() - 1;
    if interface.len() > room {
        return Err(format!("{interface} is too long a name for an ioctl"));
    }
    for (slot, byte) in request.ifr_name.iter_mut().zip(interface.bytes()) {
        *slot = byte as libc::c_char;
    }
    Ok(request)
}

fn as_sockaddr(address: Ipv4Addr) -> libc::sockaddr {
    let mut inet: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    inet.sin_family = libc::AF_INET as libc::sa_family_t;
    inet.sin_addr.s_addr = u32::from_ne_bytes(address.octets());
    unsafe { std::mem::transmute::<libc::sockaddr_in, libc::sockaddr>(inet) }
}
