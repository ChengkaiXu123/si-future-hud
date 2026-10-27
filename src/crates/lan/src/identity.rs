//! Native key stores sign TLS without exporting the private key. Only the
//! public certificate is copied into Rustls.
use super::*;
use std::{path::Path, sync::Arc};
#[derive(Clone)]
pub struct Identity {
    pub certificate: Vec<u8>,
    pub fingerprint: String,
    pub tls: Arc<rustls::ServerConfig>,
}
impl Identity {
    pub fn from_signer(
        certificate: Vec<u8>,
        key: Arc<dyn rustls::sign::SigningKey>,
    ) -> Result<Self> {
        use sha2::Digest;
        let fingerprint = format!("{:x}", sha2::Sha256::digest(&certificate));
        let certified = rustls::sign::CertifiedKey::new(
            vec![rustls::pki_types::CertificateDer::from(certificate.clone())],
            key,
        );
        let resolver = rustls::sign::SingleCertAndKey::from(certified);
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| "TLS protocol versions")?
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));
        Ok(Self {
            certificate,
            fingerprint,
            tls: Arc::new(tls),
        })
    }
    /// The host's stored TLS identity for this device.
    pub fn load(device: &str, directory: &Path) -> Result<Self> {
        host::load(device, directory)
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod certificate;

// Where the TLS identity's private key lives.
#[cfg(target_os = "macos")]
#[path = "identity/macos.rs"]
mod host;
#[cfg(target_os = "windows")]
#[path = "identity/windows.rs"]
mod host;
#[cfg(target_os = "linux")]
#[path = "identity/linux.rs"]
mod host;
