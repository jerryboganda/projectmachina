//! TLS configuration. Full chain and hostname validation are always on;
//! there is no flag anywhere in this module to disable certificate
//! verification or silently downgrade to plaintext on a TLS failure.

use std::sync::Arc;

use rustls::{ClientConfig, RootCertStore};

use crate::error::NetworkError;

/// Where the trusted root store comes from. `WebpkiRoots` (Mozilla's
/// curated bundle, vendored via the `webpki-roots` crate) is the default so
/// certificate trust is deterministic across platforms and in fixture
/// tests. `OsNativeCerts` is a placeholder for a later, explicitly opted-in
/// enterprise-proxy-root configuration -- deferred, not wired to anything
/// that can be reached from page/agent-controlled input.
#[derive(Clone, Debug, Default)]
pub enum TrustStoreSource {
    #[default]
    WebpkiRoots,
    /// Reserved for a future explicit, policy-configured custom bundle.
    /// Never populated from per-request or page-controlled input.
    Custom(Vec<Vec<u8>>),
}

pub fn build_client_config(source: &TrustStoreSource) -> Result<Arc<ClientConfig>, NetworkError> {
    let mut roots = RootCertStore::empty();
    match source {
        TrustStoreSource::WebpkiRoots => {
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
        TrustStoreSource::Custom(der_certificates) => {
            for der in der_certificates {
                let certificate = rustls_pki_types::CertificateDer::from(der.clone());
                roots
                    .add(certificate)
                    .map_err(|error| NetworkError::TlsFailed(error.to_string()))?;
            }
        }
    }

    let mut config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    // TLS 1.2 floor (prefer 1.3): rustls negotiates the highest mutually
    // supported protocol from `config.versions` in
    // `ClientConfig::builder()` defaults, which already excludes SSLv3/TLS
    // 1.0/1.1 and known-weak cipher suites (rustls never implements them).
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::{build_client_config, TrustStoreSource};

    #[test]
    fn webpki_roots_config_builds_successfully() {
        let config = build_client_config(&TrustStoreSource::WebpkiRoots).expect("config builds");
        assert_eq!(
            config.alpn_protocols,
            vec![b"h2".to_vec(), b"http/1.1".to_vec()]
        );
    }
}
