//! The connector: resolve -> evaluate_resolution -> pin one IP ->
//! evaluate_connect -> TCP connect -> (TLS handshake). Every step is
//! re-run on every redirect hop by the caller (`client.rs`); nothing here
//! is a "redirect fast path" that skips policy. Binding the connection to
//! the single already-classified `SocketAddr` (never re-resolving the
//! hostname between classification and connect) is the core of the
//! DNS-rebinding TOCTOU defense.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use rustls::ClientConfig;
use rustls_pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use url::Host;

use crate::dns::Resolver;
use crate::error::NetworkError;
use crate::policy::{NetworkPolicy, PolicyDecision, RequestMeta};

pub trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncStream for T {}

pub struct Connected {
    pub stream: Box<dyn AsyncStream>,
    pub address: SocketAddr,
    /// ALPN-negotiated protocol, if TLS was used and the peer negotiated
    /// one (`Some(b"h2")` / `Some(b"http/1.1")`). `None` for plaintext HTTP
    /// (H2C is out of scope for this pass).
    pub alpn: Option<Vec<u8>>,
}

fn policy_result(decision: PolicyDecision) -> Result<(), NetworkError> {
    match decision {
        PolicyDecision::Allow => Ok(()),
        PolicyDecision::Deny { reason, .. } => Err(NetworkError::PolicyBlocked(reason)),
    }
}

/// Resolve `host`, classify the full answer set, pin the first
/// policy-approved address, re-check that single address at connect time,
/// then dial it. `host` is the canonical `Host` already parsed out of the
/// request URL (never a raw authority string).
pub async fn connect(
    resolver: &dyn Resolver,
    policy: &dyn NetworkPolicy,
    host: &Host<String>,
    port: u16,
    tls_config: Option<Arc<ClientConfig>>,
    meta: &RequestMeta,
    connect_timeout: std::time::Duration,
) -> Result<Connected, NetworkError> {
    let addresses: Vec<IpAddr> = match host {
        Host::Ipv4(addr) => vec![IpAddr::V4(*addr)],
        Host::Ipv6(addr) => vec![IpAddr::V6(*addr)],
        Host::Domain(domain) => resolver.resolve(domain, port).await?,
    };

    policy_result(policy.evaluate_resolution(host, &addresses, meta))?;

    let pinned = *addresses
        .first()
        .ok_or_else(|| NetworkError::DnsResolutionFailed("empty answer set".to_owned()))?;
    let socket_address = SocketAddr::new(pinned, port);

    policy_result(policy.evaluate_connect(socket_address, meta))?;

    let tcp = tokio::time::timeout(connect_timeout, TcpStream::connect(socket_address))
        .await
        .map_err(|_| NetworkError::ConnectFailed("connect timed out".to_owned()))?
        .map_err(|error| NetworkError::ConnectFailed(error.to_string()))?;
    tcp.set_nodelay(true)
        .map_err(|error| NetworkError::ConnectFailed(error.to_string()))?;

    match tls_config {
        None => Ok(Connected {
            stream: Box::new(tcp),
            address: socket_address,
            alpn: None,
        }),
        Some(config) => {
            let server_name = server_name_for(host)?;
            let connector = tokio_rustls::TlsConnector::from(config);
            let tls_stream = connector
                .connect(server_name, tcp)
                .await
                .map_err(|error| NetworkError::TlsFailed(error.to_string()))?;
            let alpn = tls_stream.get_ref().1.alpn_protocol().map(|p| p.to_vec());
            Ok(Connected {
                stream: Box::new(tls_stream),
                address: socket_address,
                alpn,
            })
        }
    }
}

fn server_name_for(host: &Host<String>) -> Result<ServerName<'static>, NetworkError> {
    match host {
        Host::Domain(domain) => ServerName::try_from(domain.clone())
            .map_err(|error| NetworkError::TlsFailed(error.to_string())),
        Host::Ipv4(addr) => Ok(ServerName::IpAddress((*addr).into())),
        Host::Ipv6(addr) => Ok(ServerName::IpAddress((*addr).into())),
    }
}

#[cfg(test)]
mod tests {
    use super::connect;
    use crate::dns::StaticResolver;
    use crate::error::NetworkError;
    use crate::policy::{DenyPrivateNetworks, RequestMeta, RequestPurpose};
    use std::net::IpAddr;
    use std::time::Duration;
    use url::Host;

    fn meta() -> RequestMeta {
        RequestMeta {
            session_id: "session-1".to_owned(),
            tenant_id: None,
            purpose: RequestPurpose::Navigation,
            redirect_depth: 0,
            correlation_id: "correlation-1".to_owned(),
        }
    }

    #[tokio::test]
    async fn resolution_to_a_private_address_is_denied_before_connecting() {
        let resolver = StaticResolver::new(vec!["10.0.0.5".parse::<IpAddr>().unwrap()]);
        let policy = DenyPrivateNetworks;
        let host = Host::Domain("internal.example".to_owned());
        let result = connect(
            &resolver,
            &policy,
            &host,
            80,
            None,
            &meta(),
            Duration::from_secs(1),
        )
        .await;
        match result {
            Ok(_) => panic!("private resolved address must be denied"),
            Err(error) => assert!(matches!(error, NetworkError::PolicyBlocked(_))),
        }
    }

    #[tokio::test]
    async fn mixed_answer_set_denied_even_when_first_address_is_public() {
        let resolver = StaticResolver::new(vec![
            "93.184.216.34".parse::<IpAddr>().unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap(),
        ]);
        let policy = DenyPrivateNetworks;
        let host = Host::Domain("mixed.example".to_owned());
        let result = connect(
            &resolver,
            &policy,
            &host,
            80,
            None,
            &meta(),
            Duration::from_secs(1),
        )
        .await;
        match result {
            Ok(_) => panic!("mixed answer set must be denied"),
            Err(error) => assert!(matches!(error, NetworkError::PolicyBlocked(_))),
        }
    }
}
