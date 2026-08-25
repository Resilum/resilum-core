use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Wake {
    read: OwnedFd,
    write: OwnedFd,
    raised: AtomicBool,
}

pub enum Ready {
    Carrier,
    Woken,
}

impl Wake {
    pub fn new() -> io::Result<Self> {
        let mut ends = [0 as RawFd; 2];
        let made = unsafe { libc::pipe2(ends.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) };
        if made != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            read: unsafe { OwnedFd::from_raw_fd(ends[0]) },
            write: unsafe { OwnedFd::from_raw_fd(ends[1]) },
            raised: AtomicBool::new(false),
        })
    }

    pub fn raise(&self) {
        self.raised.store(true, Ordering::SeqCst);
        let byte = [1u8];
        unsafe {
            libc::write(self.write.as_raw_fd(), byte.as_ptr().cast(), 1);
        }
    }

    pub fn raised(&self) -> bool {
        self.raised.load(Ordering::SeqCst)
    }

    pub fn wait_for_carrier_or_a_raise(&self, carriers: &[RawFd]) -> io::Result<Ready> {
        let mut watched: Vec<libc::pollfd> = carriers
            .iter()
            .chain(std::iter::once(&self.read.as_raw_fd()))
            .map(|fd| libc::pollfd {
                fd: *fd,
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        let raiser = watched.len() - 1;
        loop {
            let waiting = u32::try_from(watched.len()).unwrap_or(u32::MAX);
            let seen = unsafe { libc::poll(watched.as_mut_ptr(), waiting as libc::nfds_t, -1) };
            if seen < 0 {
                let failure = io::Error::last_os_error();
                if failure.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(failure);
            }
            if watched[raiser].revents != 0 || self.raised() {
                return Ok(Ready::Woken);
            }
            if watched[..raiser].iter().any(|fd| fd.revents != 0) {
                return Ok(Ready::Carrier);
            }
        }
    }
}

#[cfg(test)]
mod tests;
