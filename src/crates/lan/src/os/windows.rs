//! Windows IPv4 interface discovery and protected, inheritable user ACLs.
//! The nonblocking listener polls every 25 ms, so it needs no wake-up.
use crate::Result;
use std::{
    collections::BTreeMap,
    ffi::c_void,
    fs,
    net::{Ipv4Addr, TcpListener},
    os::windows::{ffi::OsStrExt, fs::MetadataExt},
    path::Path,
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, LocalFree, ERROR_ALREADY_EXISTS, ERROR_BUFFER_OVERFLOW,
        ERROR_SUCCESS, HANDLE,
    },
    NetworkManagement::{
        IpHelper::{
            GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
            GAA_FLAG_SKIP_MULTICAST, IF_TYPE_ETHERNET_CSMACD, IF_TYPE_IEEE80211,
            IF_TYPE_SOFTWARE_LOOPBACK, IP_ADAPTER_ADDRESSES_LH,
        },
        Ndis::IfOperStatusUp,
    },
    Networking::WinSock::{AF_INET, SOCKADDR_IN},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetNamedSecurityInfoW, SetNamedSecurityInfoW, SE_FILE_OBJECT,
        },
        GetSecurityDescriptorDacl, GetTokenInformation, TokenUser, DACL_SECURITY_INFORMATION,
        OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, SECURITY_ATTRIBUTES,
        TOKEN_QUERY, TOKEN_USER,
    },
    Storage::FileSystem::{CreateDirectoryW, FILE_ATTRIBUTE_REPARSE_POINT},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub(crate) enum Wait {
    Accept,
    #[allow(dead_code)]
    Stop,
    #[allow(dead_code)]
    Retry,
}

pub(crate) struct Waiter;
pub(crate) struct Wake;

pub(crate) fn wake_pair() -> Result<(Waiter, Wake)> {
    Ok((Waiter, Wake))
}

impl Wake {
    pub(crate) fn wake(self) {}
}

/// The nonblocking accept itself polls; the stop flag ends the loop.
pub(crate) fn wait(_listener: &TcpListener, _waiter: &Waiter) -> Wait {
    Wait::Accept
}

pub(crate) fn addresses() -> Vec<String> {
    adapters()
        .into_iter()
        .map(|(_, ip)| ip.to_string())
        .collect()
}

/// Tunnel adapters' addresses in the overlay range, such as Tailscale's.
pub(crate) fn overlay_addresses() -> Vec<String> {
    overlay_only(adapters())
}

fn overlay_only(rows: Vec<((bool, u32), Ipv4Addr)>) -> Vec<String> {
    rows.into_iter()
        .filter(|((tunnel, _), ip)| *tunnel && super::overlay(*ip))
        .map(|(_, ip)| ip.to_string())
        .collect()
}

/// IPv4 addresses ranked Ethernet/Wi-Fi first, then by interface metric. The
/// rank's flag is true for any other adapter type.
fn adapters() -> Vec<((bool, u32), Ipv4Addr)> {
    // Microsoft recommends a 15 KB initial buffer. A changing adapter list can
    // require another call; u64 storage preserves the native structs' alignment.
    let mut size = 15_000u32;
    for _ in 0..3 {
        let mut buffer = vec![0u64; (size as usize).div_ceil(size_of::<u64>())];
        let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let status = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_INET),
                GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER,
                std::ptr::null(),
                first,
                &mut size,
            )
        };
        if status == ERROR_SUCCESS {
            return unsafe { collect_addresses(first) };
        }
        if status != ERROR_BUFFER_OVERFLOW {
            break;
        }
    }
    Vec::new()
}

unsafe fn collect_addresses(
    mut at: *const IP_ADAPTER_ADDRESSES_LH,
) -> Vec<((bool, u32), Ipv4Addr)> {
    let mut rows = BTreeMap::new();
    while let Some(adapter) = unsafe { at.as_ref() } {
        if adapter.OperStatus == IfOperStatusUp && adapter.IfType != IF_TYPE_SOFTWARE_LOOPBACK {
            let rank = (
                !matches!(adapter.IfType, IF_TYPE_ETHERNET_CSMACD | IF_TYPE_IEEE80211),
                adapter.Ipv4Metric,
            );
            let mut unicast = adapter.FirstUnicastAddress;
            while let Some(address) = unsafe { unicast.as_ref() } {
                let socket = address.Address.lpSockaddr;
                if !socket.is_null()
                    && address.Address.iSockaddrLength >= size_of::<SOCKADDR_IN>() as i32
                    && unsafe { (*socket).sa_family } == AF_INET
                {
                    let address = unsafe { &*socket.cast::<SOCKADDR_IN>() };
                    let ip = Ipv4Addr::from(unsafe { address.sin_addr.S_un.S_addr }.to_ne_bytes());
                    if !ip.is_loopback()
                        && !ip.is_unspecified()
                        && !ip.is_multicast()
                        && !ip.is_broadcast()
                    {
                        rows.entry(ip)
                            .and_modify(|previous: &mut (bool, u32)| {
                                *previous = (*previous).min(rank)
                            })
                            .or_insert(rank);
                    }
                }
                unicast = address.Next;
            }
        }
        at = adapter.Next;
    }
    let mut rows: Vec<_> = rows.into_iter().map(|(ip, rank)| (rank, ip)).collect();
    rows.sort_unstable();
    rows
}

struct Local(*mut c_void);
impl Drop for Local {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { LocalFree(self.0) };
        }
    }
}

unsafe fn sid_string(sid: *mut c_void) -> Result<String> {
    let mut text = std::ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err("The LAN directory owner is unavailable".into());
    }
    let text = Local(text.cast());
    let mut length = 0;
    while unsafe { *text.0.cast::<u16>().add(length) } != 0 {
        length += 1;
    }
    Ok(String::from_utf16_lossy(unsafe {
        std::slice::from_raw_parts(text.0.cast::<u16>(), length)
    }))
}

fn current_user() -> Result<String> {
    let mut token: HANDLE = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err("The current Windows user token is unavailable".into());
    }
    let mut size = 0;
    unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size) };
    let mut buffer = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        )
    };
    unsafe { CloseHandle(token) };
    if ok == 0 {
        return Err("The current Windows user identity is unavailable".into());
    }
    unsafe { sid_string((*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid) }
}

fn directory_security(sid: &str) -> Result<Local> {
    // Protected: remove inherited broad grants. Both file and subdirectory
    // inheritance keep every newly written credential private as well.
    // An explicit owner also avoids Administrators becoming a new directory's
    // default owner when this user's process is running elevated.
    let sddl: Vec<u16> = format!("O:{sid}D:P(A;OICI;GA;;;{sid})(A;OICI;GA;;;SY)")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err("The private LAN directory ACL could not be created".into());
    }
    Ok(Local(descriptor))
}

pub(crate) fn create_directory(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        // A later store can protect one of these ancestors too. Give newly
        // created parents the same user ownership even in an elevated process.
        if !parent.try_exists().map_err(|e| e.to_string())? {
            create_directory(parent)?;
        }
    }
    let descriptor = directory_security(&current_user()?)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { CreateDirectoryW(path.as_ptr(), &attributes) } == 0
        && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
    {
        return Err("The private LAN directory could not be created".into());
    }
    Ok(())
}

pub(crate) fn protect_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err("The LAN credential directory must be a real directory".into());
    }
    let sid = current_user()?;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut owner = std::ptr::null_mut();
    let mut descriptor = std::ptr::null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut descriptor,
        )
    };
    let _owner = Local(descriptor);
    if status != ERROR_SUCCESS || owner.is_null() || unsafe { sid_string(owner) }? != sid {
        return Err("The LAN credential directory must belong to the current user".into());
    }
    let descriptor = directory_security(&sid)?;
    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl = std::ptr::null_mut();
    if unsafe { GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut dacl, &mut defaulted) }
        == 0
        || present == 0
        || dacl.is_null()
    {
        return Err("The private LAN directory ACL is unavailable".into());
    }
    if unsafe {
        SetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            dacl,
            std::ptr::null(),
        )
    } != ERROR_SUCCESS
    {
        return Err("The private LAN directory ACL could not be applied".into());
    }
    Ok(())
}

/// New credential files inherit the containing directory's protected user ACL.
pub(crate) fn private_file(_options: &mut fs::OpenOptions) {}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::{
        NetworkManagement::IpHelper::IP_ADAPTER_UNICAST_ADDRESS_LH,
        Networking::WinSock::SOCKET_ADDRESS,
        Security::{GetAce, GetSecurityDescriptorControl, ACCESS_ALLOWED_ACE, SE_DACL_PROTECTED},
    };

    #[test]
    fn adapter_addresses_exclude_loopback_and_prefer_physical_lan_interfaces() {
        let mut sockets = [
            [10, 0, 0, 8],
            [192, 168, 1, 9],
            [127, 0, 0, 1],
            [10, 0, 0, 8],
            [100, 69, 88, 16],
        ]
        .map(|bytes| {
            let mut socket = SOCKADDR_IN {
                sin_family: AF_INET,
                ..Default::default()
            };
            socket.sin_addr.S_un.S_addr = u32::from_ne_bytes(bytes);
            socket
        });
        let mut unicast: Vec<_> = sockets
            .iter_mut()
            .map(|socket| IP_ADAPTER_UNICAST_ADDRESS_LH {
                Address: SOCKET_ADDRESS {
                    lpSockaddr: (socket as *mut SOCKADDR_IN).cast(),
                    iSockaddrLength: size_of::<SOCKADDR_IN>() as i32,
                },
                ..Default::default()
            })
            .collect();
        let mut adapters: Vec<_> = unicast
            .iter_mut()
            .enumerate()
            .map(|(index, address)| IP_ADAPTER_ADDRESSES_LH {
                FirstUnicastAddress: address,
                OperStatus: IfOperStatusUp,
                IfType: if index == 1 { IF_TYPE_IEEE80211 } else { 131 }, // tunnel
                ..Default::default()
            })
            .collect();
        for index in 0..adapters.len() - 1 {
            adapters[index].Next = &mut adapters[index + 1];
        }
        let ips = |rows: Vec<((bool, u32), Ipv4Addr)>| {
            rows.into_iter()
                .map(|(_, ip)| ip.to_string())
                .collect::<Vec<_>>()
        };
        let rows = unsafe { collect_addresses(adapters.as_ptr()) };
        assert_eq!(
            ips(rows.clone()),
            ["192.168.1.9", "10.0.0.8", "100.69.88.16"]
        );
        assert_eq!(overlay_only(rows), ["100.69.88.16"]);
        adapters[1].OperStatus = 2; // IfOperStatusDown
        assert_eq!(
            ips(unsafe { collect_addresses(adapters.as_ptr()) }),
            ["10.0.0.8", "100.69.88.16"]
        );
    }

    fn access_entries(path: &Path) -> (u16, Vec<String>) {
        let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut descriptor = std::ptr::null_mut();
        let mut dacl = std::ptr::null_mut();
        assert_eq!(
            unsafe {
                GetNamedSecurityInfoW(
                    path.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut dacl,
                    std::ptr::null_mut(),
                    &mut descriptor,
                )
            },
            ERROR_SUCCESS
        );
        let descriptor = Local(descriptor);
        let mut control = 0;
        let mut revision = 0;
        assert_ne!(
            unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) },
            0
        );
        assert!(!dacl.is_null());
        let mut entries = Vec::new();
        for index in 0..unsafe { (*dacl).AceCount } {
            let mut ace = std::ptr::null_mut();
            assert_ne!(unsafe { GetAce(dacl, u32::from(index), &mut ace) }, 0);
            let ace = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
            assert_eq!(ace.Header.AceType, 0); // ACCESS_ALLOWED_ACE_TYPE
            entries.push(
                unsafe { sid_string((&ace.SidStart as *const u32).cast_mut().cast()) }.unwrap(),
            );
        }
        entries.sort();
        // Windows can split a generic inheritable grant into effective and
        // inherit-only ACEs. Compare the granted identities, not the ACE count.
        entries.dedup();
        (control, entries)
    }

    #[test]
    fn private_directory_protects_its_acl_and_new_files_inherit_it() {
        struct Fixture(std::path::PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let root =
            Fixture(std::env::temp_dir().join(format!("agenthud-acl-{}", uuid::Uuid::new_v4())));
        // A child store can be initialized before its parents are used as
        // stores. Every newly created directory must have TokenUser ownership
        // even in an elevated process; existing parents are left untouched.
        let nested = root.0.join("data").join("messaging");
        create_directory(&nested).unwrap();
        protect_directory(&nested).unwrap();
        crate::fs::private_dir(nested.parent().unwrap()).unwrap();
        crate::fs::private_dir(&root.0).unwrap();
        let (control, entries) = access_entries(&root.0);
        let mut expected = vec![current_user().unwrap(), "S-1-5-18".into()];
        expected.sort();
        assert_ne!(control & SE_DACL_PROTECTED, 0);
        assert_eq!(entries, expected);
        let file = root.0.join("clients.json");
        crate::fs::atomic_json(&file, &serde_json::json!({"first":true})).unwrap();
        crate::fs::atomic_json(&file, &serde_json::json!({"second":true})).unwrap();
        assert_eq!(access_entries(&file).1, expected);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&file).unwrap()).unwrap(),
            serde_json::json!({"second":true})
        );
    }
}
