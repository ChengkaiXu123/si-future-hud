//! The LAN TLS identity: not implemented on this host.
//!
//! On macOS the signing key is created in the Keychain and never exported.
//! An adapter here would keep it in the Secret Service or a TPM-backed key and
//! return the same `Identity` through `Identity::from_signer`.
use super::*;

pub(super) fn load(_device: &str, _directory: &Path) -> Result<Identity> {
    Err("This host has no installed secure LAN identity adapter".into())
}
