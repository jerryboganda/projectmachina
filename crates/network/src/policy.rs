//! The network-policy callback hook (M2-T02 design section 5). `network`
//! defines the trait and ships a conservative fail-closed default
//! (`DenyPrivateNetworks`); `security-policy`/`native-core` compose richer
//! policy on top later (M7-T04). `network` never depends on
//! `machina-security-policy` -- the dependency is inverted.

use std::net::IpAddr;

use url::Host;

use crate::address_class::classify;
use crate::error::DenyReason;
use crate::ports::is_blocked_port;
use crate::url::NormalizedUrl;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestPurpose {
    Navigation,
    Subresource,
    Fetch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestMeta {
    pub session_id: String,
    pub tenant_id: Option<String>,
    pub purpose: RequestPurpose,
    pub redirect_depth: u32,
    pub correlation_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: DenyReason, message: String },
}

impl PolicyDecision {
    pub fn deny(reason: DenyReason) -> Self {
        let message = reason.to_string();
        Self::Deny { reason, message }
    }

    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// The three call sites the redirect loop re-runs on every hop (design
/// section 5): URL syntax, full DNS answer set, and the single pinned
/// address actually dialed. All three are synchronous/non-blocking: any
/// dynamic policy source (rate limits, tenant config) must be pre-fetched
/// or cached by the implementer, never awaited here.
pub trait NetworkPolicy: Send + Sync {
    fn evaluate_url(&self, url: &NormalizedUrl, meta: &RequestMeta) -> PolicyDecision;
    fn evaluate_resolution(
        &self,
        host: &Host<String>,
        addresses: &[IpAddr],
        meta: &RequestMeta,
    ) -> PolicyDecision;
    fn evaluate_connect(&self, address: std::net::SocketAddr, meta: &RequestMeta)
        -> PolicyDecision;
}

/// Conservative built-in default: deny loopback, RFC1918, link-local
/// (including cloud metadata addresses), CGNAT, unique-local IPv6,
/// reserved/special-use ranges, multicast/broadcast, and blocked ports.
/// This is the fail-closed base every composed policy starts from, and is
/// sufficient on its own to satisfy the M2-T02 acceptance criterion even
/// before `security-policy` (M7-T04) is populated.
#[derive(Clone, Copy, Debug, Default)]
pub struct DenyPrivateNetworks;

impl DenyPrivateNetworks {
    fn classify_address(address: IpAddr) -> PolicyDecision {
        use crate::address_class::DestinationClass;
        let class = classify(address);
        if class.is_safe_default() {
            return PolicyDecision::Allow;
        }
        let reason = match class {
            DestinationClass::Public => unreachable!("handled above"),
            DestinationClass::Loopback => DenyReason::Loopback,
            DestinationClass::LinkLocal => DenyReason::LinkLocal,
            DestinationClass::CloudMetadata => DenyReason::CloudMetadata,
            DestinationClass::Private => DenyReason::PrivateNetwork,
            DestinationClass::UniqueLocalV6 => DenyReason::UniqueLocalV6,
            DestinationClass::CarrierGradeNat => DenyReason::CarrierGradeNat,
            DestinationClass::Reserved => DenyReason::ReservedRange,
            DestinationClass::Multicast => DenyReason::Multicast,
            DestinationClass::Broadcast => DenyReason::Broadcast,
            DestinationClass::Unspecified => DenyReason::UnspecifiedAddress,
        };
        PolicyDecision::deny(reason)
    }
}

impl NetworkPolicy for DenyPrivateNetworks {
    fn evaluate_url(&self, url: &NormalizedUrl, _meta: &RequestMeta) -> PolicyDecision {
        if url.has_embedded_credentials() {
            return PolicyDecision::deny(DenyReason::EmbeddedCredentials);
        }
        if is_blocked_port(url.port()) {
            return PolicyDecision::deny(DenyReason::BlockedPort);
        }
        // A literal IP host is classified immediately so an obviously bad
        // literal-IP URL (e.g. `http://169.254.169.254/`) is rejected before
        // any DNS call, per the security review's pipeline step 2.
        match url.host() {
            Host::Ipv4(addr) => Self::classify_address(IpAddr::V4(addr)),
            Host::Ipv6(addr) => Self::classify_address(IpAddr::V6(addr)),
            Host::Domain(_) => PolicyDecision::Allow,
        }
    }

    fn evaluate_resolution(
        &self,
        _host: &Host<String>,
        addresses: &[IpAddr],
        _meta: &RequestMeta,
    ) -> PolicyDecision {
        // Deny if *any* candidate violates policy -- never pick the first
        // good one and ignore the rest (mixed safe/unsafe answer defense).
        for address in addresses {
            if let PolicyDecision::Deny { reason, .. } = Self::classify_address(*address) {
                return PolicyDecision::Deny {
                    reason: DenyReason::MixedAnswerSet,
                    message: format!("{address} classified as {reason}"),
                };
            }
        }
        PolicyDecision::Allow
    }

    fn evaluate_connect(
        &self,
        address: std::net::SocketAddr,
        _meta: &RequestMeta,
    ) -> PolicyDecision {
        if is_blocked_port(address.port()) {
            return PolicyDecision::deny(DenyReason::BlockedPort);
        }
        Self::classify_address(address.ip())
    }
}

#[cfg(test)]
mod tests {
    use super::{DenyPrivateNetworks, NetworkPolicy, PolicyDecision, RequestMeta, RequestPurpose};
    use crate::url::NormalizedUrl;
    use std::net::{IpAddr, SocketAddr};
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

    #[test]
    fn literal_metadata_ip_denied_before_any_dns_call() {
        let policy = DenyPrivateNetworks;
        let url = NormalizedUrl::parse("http://169.254.169.254/latest/meta-data/", None)
            .expect("valid url");
        let decision = policy.evaluate_url(&url, &meta());
        assert!(!decision.is_allow());
    }

    #[test]
    fn mixed_answer_set_is_denied_even_with_one_public_address() {
        let policy = DenyPrivateNetworks;
        let addresses: Vec<IpAddr> = vec![
            "93.184.216.34".parse().unwrap(),
            "127.0.0.1".parse().unwrap(),
        ];
        let host = Host::Domain("example.com".to_owned());
        let decision = policy.evaluate_resolution(&host, &addresses, &meta());
        assert!(
            !decision.is_allow(),
            "one bad address must deny the whole answer set"
        );
    }

    #[test]
    fn public_domain_allowed_at_url_stage_pending_resolution() {
        let policy = DenyPrivateNetworks;
        let url = NormalizedUrl::parse("http://example.com/", None).expect("valid url");
        assert!(policy.evaluate_url(&url, &meta()).is_allow());
    }

    #[test]
    fn connect_stage_rechecks_the_pinned_address() {
        let policy = DenyPrivateNetworks;
        let address: SocketAddr = "127.0.0.1:80".parse().unwrap();
        assert!(!policy.evaluate_connect(address, &meta()).is_allow());
        let address: SocketAddr = "93.184.216.34:443".parse().unwrap();
        assert!(policy.evaluate_connect(address, &meta()).is_allow());
    }

    #[test]
    fn embedded_credentials_are_denied() {
        let policy = DenyPrivateNetworks;
        let url = NormalizedUrl::parse("http://user:pass@example.com/", None).expect("valid url");
        assert!(matches!(
            policy.evaluate_url(&url, &meta()),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn blocked_port_denied_at_url_and_connect_stages() {
        let policy = DenyPrivateNetworks;
        let url = NormalizedUrl::parse("http://example.com:25/", None).expect("valid url");
        assert!(matches!(
            policy.evaluate_url(&url, &meta()),
            PolicyDecision::Deny { .. }
        ));
        let address: SocketAddr = "93.184.216.34:25".parse().unwrap();
        assert!(matches!(
            policy.evaluate_connect(address, &meta()),
            PolicyDecision::Deny { .. }
        ));
    }
}
