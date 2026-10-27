//! Operating-system pieces of the LAN link. unix.rs (macOS and Linux) and
//! windows.rs provide the same items, and the rest of the crate never branches
//! on the operating system:
//!   wake_pair, Wake, Waiter, wait, Wait   listener cancellation
//!   addresses                             this device's LAN IPv4 addresses
//!   overlay_addresses                     its Tailscale-style tunnel IPv4 addresses
//!   create_directory, protect_directory,  owner-only credentials on disk
//!     private_file
#[cfg(unix)]
#[path = "unix.rs"]
mod host;
#[cfg(windows)]
#[path = "windows.rs"]
mod host;
pub(crate) use host::*;

/// 100.64.0.0/10: the shared address space that Tailscale (and similar overlay
/// networks) assigns to each device on its tunnel interface.
pub(crate) fn overlay(ip: std::net::Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 100 && (64..128).contains(&b)
}

#[cfg(test)]
mod tests {
    use super::overlay;
    use std::net::Ipv4Addr;

    #[test]
    fn overlay_is_the_shared_address_space_only() {
        assert!(overlay(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(overlay(Ipv4Addr::new(100, 69, 88, 16)));
        assert!(overlay(Ipv4Addr::new(100, 127, 255, 254)));
        assert!(!overlay(Ipv4Addr::new(100, 63, 255, 255)));
        assert!(!overlay(Ipv4Addr::new(100, 128, 0, 1)));
        assert!(!overlay(Ipv4Addr::new(192, 168, 1, 19)));
        assert!(!overlay(Ipv4Addr::new(198, 18, 0, 1)));
    }
}
