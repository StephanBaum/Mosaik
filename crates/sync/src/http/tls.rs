//! The TLS trust of every HTTPS client of this crate (ADR-0091, M10).
//!
//! On the desktop, in the CLI and on iOS the trust is the operating system's
//! (`rustls-platform-verifier`). On Android that crate needs JNI
//! initialisation, which means hand-written `unsafe`, so the platform passes the
//! trust anchors it collected (`TrustManagerFactory` with the app's network
//! security config) and [`TlsTrust::Anchors`] verifies against them with
//! rustls' own `WebPKI` verifier. Either way: the `ring` provider, TLS 1.3 only,
//! no client certificate.

use std::sync::{Arc, OnceLock};

use rustls::client::WebPkiServerVerifier;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::CertificateDer;
use rustls::{ClientConfig, RootCertStore};

use super::client::tls_versions;
use crate::transport::TransportError;

/// The most anchors accepted (the Android system store has about 150).
pub const MAX_TRUST_ANCHORS: usize = 1024;

/// Where the TLS trust comes from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TlsTrust {
    /// The operating system's verifier (not available on Android).
    #[cfg(not(target_os = "android"))]
    Platform,
    /// These trust anchors (DER certificates), and nothing else.
    Anchors(Vec<CertificateDer<'static>>),
}

static TRUST: OnceLock<TlsTrust> = OnceLock::new();

/// Sets the process-wide TLS trust. Call it once, before the first
/// connection. Setting the same trust again is accepted; a different one is
/// refused, so the trust cannot change under a running client.
///
/// # Errors
/// [`TransportError::Io`] for no anchors, more than [`MAX_TRUST_ANCHORS`], a
/// certificate that is not a valid trust anchor, or a different trust than the
/// one already set.
pub fn set_tls_trust(trust: TlsTrust) -> Result<(), TransportError> {
    if let TlsTrust::Anchors(anchors) = &trust {
        verifier(anchors, &provider())?;
    }
    match TRUST.set(trust) {
        Ok(()) => Ok(()),
        Err(trust) if TRUST.get() == Some(&trust) => Ok(()),
        Err(_) => Err(TransportError::Io),
    }
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn verifier(
    anchors: &[CertificateDer<'static>],
    provider: &Arc<CryptoProvider>,
) -> Result<Arc<WebPkiServerVerifier>, TransportError> {
    if anchors.is_empty() || anchors.len() > MAX_TRUST_ANCHORS {
        return Err(TransportError::Io);
    }
    let mut roots = RootCertStore::empty();
    for anchor in anchors {
        roots.add(anchor.clone()).map_err(|_| TransportError::Io)?;
    }
    WebPkiServerVerifier::builder_with_provider(Arc::new(roots), Arc::clone(provider))
        .build()
        .map_err(|_| TransportError::Io)
}

/// The trust in force: the one that was set, else the platform's (not on
/// Android, where nothing is trusted until the platform passes its anchors).
#[cfg_attr(
    not(target_os = "android"),
    expect(
        clippy::unnecessary_wraps,
        reason = "Android has no default trust, so the same signature must fit it"
    )
)]
fn current() -> Option<TlsTrust> {
    if let Some(trust) = TRUST.get() {
        return Some(trust.clone());
    }
    #[cfg(not(target_os = "android"))]
    {
        Some(TlsTrust::Platform)
    }
    #[cfg(target_os = "android")]
    {
        None
    }
}

/// The client configuration for the trust in force.
pub(super) fn config() -> Result<ClientConfig, TransportError> {
    config_for(&current().ok_or(TransportError::Io)?)
}

fn config_for(trust: &TlsTrust) -> Result<ClientConfig, TransportError> {
    let provider = provider();
    let builder = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_protocol_versions(tls_versions())
        .map_err(|_| TransportError::Io)?;
    Ok(match trust {
        #[cfg(not(target_os = "android"))]
        TlsTrust::Platform => {
            use rustls_platform_verifier::BuilderVerifierExt as _;
            builder
                .with_platform_verifier()
                .map_err(|_| TransportError::Io)?
                .with_no_client_auth()
        }
        TlsTrust::Anchors(anchors) => builder
            .with_webpki_verifier(verifier(anchors, &provider)?)
            .with_no_client_auth(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A self-signed P-256 CA certificate ("Mosaic Test Root"), generated for
    /// these tests only.
    const TEST_ROOT_HEX: &str = include_str!("../../tests/data/test-root.hex");

    fn test_root() -> CertificateDer<'static> {
        let hex = TEST_ROOT_HEX.trim();
        let bytes: Vec<u8> = (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
            .collect();
        CertificateDer::from(bytes)
    }

    #[test]
    fn anchors_trust_is_tls13_only_with_ring() {
        let config = config_for(&TlsTrust::Anchors(vec![test_root()])).unwrap();
        // rustls exposes no accessor for the versions; `config_for` takes them
        // from `tls_versions()`, which the client tests pin to TLS 1.3 only.
        assert_eq!(tls_versions().len(), 1);
        assert!(!config.client_auth_cert_resolver.has_certs());
        // The setter validates the anchors before it stores them, so a bad
        // set never changes the process-wide trust.
        assert_eq!(
            set_tls_trust(TlsTrust::Anchors(vec![CertificateDer::from(vec![1])])),
            Err(TransportError::Io)
        );
    }

    #[test]
    fn anchors_trust_without_anchors_is_refused() {
        assert_eq!(
            config_for(&TlsTrust::Anchors(Vec::new())).err(),
            Some(TransportError::Io)
        );
        assert_eq!(
            set_tls_trust(TlsTrust::Anchors(Vec::new())),
            Err(TransportError::Io)
        );
        // Garbage is not a trust anchor, and one bad anchor refuses the set.
        let garbage = CertificateDer::from(vec![1, 2, 3]);
        assert_eq!(
            config_for(&TlsTrust::Anchors(vec![test_root(), garbage])).err(),
            Some(TransportError::Io)
        );
    }
}
