//! POSIX pieces of the LAN link: a poll(2) wake-up for the listener, IPv4
//! interface addresses, and owner-only files.
use crate::Result;
use std::{
    fs,
    io::Write,
    net::TcpListener,
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::Path,
};

/// What a blocked listener should do next.
pub(crate) enum Wait {
    Accept,
    Stop,
    Retry,
}

/// The listener side of a cancellation pair.
pub(crate) struct Waiter(UnixStream);
/// The owner side: waking it ends the listener's wait.
pub(crate) struct Wake(UnixStream);

pub(crate) fn wake_pair() -> Result<(Waiter, Wake)> {
    let (read, write) = UnixStream::pair().map_err(|_| "LAN listener cancellation pair")?;
    Ok((Waiter(read), Wake(write)))
}

impl Wake {
    pub(crate) fn wake(mut self) {
        let _ = self.0.write_all(&[1]);
    }
}

/// Blocks until a connection arrives or the owner wakes the listener.
pub(crate) fn wait(listener: &TcpListener, waiter: &Waiter) -> Wait {
    let mut fds = [
        libc::pollfd {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: waiter.0.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    let result = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) };
    if result < 0 {
        return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            Wait::Retry
        } else {
            Wait::Stop
        };
    }
    if fds[1].revents != 0 || fds[0].revents & libc::POLLIN == 0 {
        return Wait::Stop;
    }
    Wait::Accept
}

/// Up, non-loopback IPv4 addresses of Ethernet/Wi-Fi (`en*`) interfaces first,
/// then bridges.
pub(crate) fn addresses() -> Vec<String> {
    let mut rows: Vec<_> = interfaces()
        .into_iter()
        .filter(|(name, _)| name.starts_with("en") || name.starts_with("bridge"))
        .map(|(name, ip)| (!name.starts_with("en"), name, ip.to_string()))
        .collect();
    rows.sort();
    rows.into_iter().map(|(_, _, ip)| ip).collect()
}

/// Tunnel addresses in the overlay range: Tailscale's `utun*` on macOS,
/// `tailscale*` on Linux. A proxy's tunnel (198.18.0.0/15) is not one.
pub(crate) fn overlay_addresses() -> Vec<String> {
    let mut rows: Vec<_> = interfaces()
        .into_iter()
        .filter(|(name, ip)| {
            (name.starts_with("utun") || name.starts_with("tailscale")) && super::overlay(*ip)
        })
        .map(|(name, ip)| (name, ip.to_string()))
        .collect();
    rows.sort();
    rows.into_iter().map(|(_, ip)| ip).collect()
}

/// Up, non-loopback IPv4 addresses with their interface names.
fn interfaces() -> Vec<(String, std::net::Ipv4Addr)> {
    unsafe {
        let mut first = std::ptr::null_mut();
        if libc::getifaddrs(&mut first) != 0 {
            return Vec::new();
        }
        let mut rows = Vec::new();
        let mut at = first;
        while let Some(entry) = at.as_ref() {
            if !entry.ifa_addr.is_null()
                && (*entry.ifa_addr).sa_family as i32 == libc::AF_INET
                && entry.ifa_flags as i32 & libc::IFF_UP != 0
                && entry.ifa_flags as i32 & libc::IFF_LOOPBACK == 0
            {
                let name = std::ffi::CStr::from_ptr(entry.ifa_name).to_string_lossy();
                let addr = &*(entry.ifa_addr as *const libc::sockaddr_in);
                let ip = std::net::Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes());
                if !ip.is_loopback() && !ip.is_unspecified() {
                    rows.push((name.into_owned(), ip));
                }
            }
            at = entry.ifa_next;
        }
        libc::freeifaddrs(first);
        rows
    }
}

pub(crate) fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| e.to_string())
}

pub(crate) fn protect_directory(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}

pub(crate) fn private_file(options: &mut fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}
