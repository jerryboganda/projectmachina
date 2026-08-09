//! Shared test-only helpers for `machina-network`'s integration suites.
//! Nothing here is exported from the crate's production API. Each
//! integration test binary compiles this module independently and uses a
//! different subset of it, so an unused-item warning here is expected
//! (`#[allow(dead_code)]`) rather than a real signal of dead production
//! code.
#![allow(dead_code)]

pub mod fixture_process;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use machina_command_bus::CommandContext;
use machina_network::{
    ClientConfigOptions, NetworkClient, NetworkPolicy, PolicyDecision, RequestMeta, SystemResolver,
    TrustStoreSource, UnlimitedBudget,
};
use url::Host;

/// A test-only policy that allows every destination, including loopback --
/// production code must never use this; the default (`DenyPrivateNetworks`)
/// is exercised directly in `policy_rejection.rs` and unit tests, not
/// bypassed here. This exists solely so the fixture-navigation and
/// malformed-response suites can point the loader at loopback fixture
/// servers.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllPolicy;

impl NetworkPolicy for AllowAllPolicy {
    fn evaluate_url(
        &self,
        _url: &machina_network::NormalizedUrl,
        _meta: &RequestMeta,
    ) -> PolicyDecision {
        PolicyDecision::Allow
    }

    fn evaluate_resolution(
        &self,
        _host: &Host<String>,
        _addresses: &[IpAddr],
        _meta: &RequestMeta,
    ) -> PolicyDecision {
        PolicyDecision::Allow
    }

    fn evaluate_connect(&self, _address: SocketAddr, _meta: &RequestMeta) -> PolicyDecision {
        PolicyDecision::Allow
    }
}

pub fn test_client() -> NetworkClient {
    test_client_with_options(ClientConfigOptions::default())
}

pub fn test_client_with_options(options: ClientConfigOptions) -> NetworkClient {
    NetworkClient::new(
        Arc::new(SystemResolver),
        Arc::new(AllowAllPolicy),
        Arc::new(UnlimitedBudget),
        machina_network::tls::build_client_config(&TrustStoreSource::WebpkiRoots)
            .expect("default tls config builds"),
        options,
    )
    .expect("client construction succeeds")
}

pub fn test_ctx() -> CommandContext {
    CommandContext::with_timeout("test-correlation", Duration::from_secs(10))
}

pub fn test_ctx_with_timeout(timeout: Duration) -> CommandContext {
    CommandContext::with_timeout("test-correlation", timeout)
}
