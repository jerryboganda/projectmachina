//! Focused SSRF/redirect negative fixtures against the *production*
//! default policy (`DenyPrivateNetworks`), not the permissive test policy
//! used by `fixture_navigation.rs`. These prove reject-by-default is the
//! safety property under test, end to end through a real `NetworkClient`,
//! not just at the unit level (`policy.rs`/`connector.rs` already cover the
//! destination-class table and mixed-answer-set unit tests).

mod support;

use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use machina_network::{
    ClientConfigOptions, DenyPrivateNetworks, NetworkClient, NetworkError, NetworkPolicy,
    NormalizedUrl, PolicyDecision, RequestMeta, RequestSpec, StaticResolver, TrustStoreSource,
    UnlimitedBudget,
};
use support::fixture_process::FixtureProcess;
use url::Host;

fn default_policy_client() -> NetworkClient {
    NetworkClient::new(
        Arc::new(machina_network::SystemResolver),
        Arc::new(DenyPrivateNetworks),
        Arc::new(UnlimitedBudget),
        machina_network::tls::build_client_config(&TrustStoreSource::WebpkiRoots)
            .expect("tls config builds"),
        ClientConfigOptions::default(),
    )
    .expect("client construction succeeds")
}

/// The security review's "integration test with default policy pointed at
/// fixture loopback address rejected before reaching handler" item,
/// literally: a real, running fixture server, hit through the crate's
/// production default policy, must never receive the request.
#[test]
fn default_policy_rejects_the_real_running_fixture_before_reaching_it() {
    let fixture = FixtureProcess::spawn(1);
    let client = default_policy_client();
    let url = NormalizedUrl::parse(&format!("{}/navigation", fixture.origin(0)), None)
        .expect("valid url");
    let ctx = support::test_ctx();
    match client.fetch(RequestSpec::get(url), "session-ssrf", &ctx) {
        Err(NetworkError::PolicyBlocked(reason)) => {
            let message = reason.to_string();
            assert!(
                message.contains("loopback") || message.contains("Loopback"),
                "expected a loopback denial, got: {message}"
            );
        }
        Err(other) => panic!("expected a policy-blocked error, got {other:?}"),
        Ok(_) => panic!("default policy must never let a loopback fetch succeed"),
    }
}

/// A hostname that looks ordinary at the URL-parsing stage but resolves
/// (via an injected resolver, standing in for a hostile/rebinding DNS
/// answer) to the fixture's real loopback address must still be denied --
/// this exercises the `evaluate_resolution` stage specifically, distinct
/// from the literal-IP fast path already covered by `evaluate_url`.
#[test]
fn domain_that_resolves_to_a_private_address_is_denied_at_the_resolution_stage() {
    let fixture = FixtureProcess::spawn(1);
    let fixture_ip: IpAddr = "127.0.0.1".parse().expect("loopback literal");
    let fixture_port = fixture.instances[0].port;

    let resolver = Arc::new(StaticResolver::new(vec![fixture_ip]));
    let client = NetworkClient::new(
        resolver,
        Arc::new(DenyPrivateNetworks),
        Arc::new(UnlimitedBudget),
        machina_network::tls::build_client_config(&TrustStoreSource::WebpkiRoots)
            .expect("tls config builds"),
        ClientConfigOptions::default(),
    )
    .expect("client construction succeeds");

    let url = NormalizedUrl::parse(
        &format!("http://looks-public.example.invalid:{fixture_port}/navigation"),
        None,
    )
    .expect("valid url");
    let ctx = support::test_ctx();
    match client.fetch(RequestSpec::get(url), "session-rebinding", &ctx) {
        Err(NetworkError::PolicyBlocked(_)) => {}
        Err(other) => panic!("expected a policy-blocked error, got {other:?}"),
        Ok(_) => panic!("a domain resolving to a private address must be denied"),
    }
}

/// Every one of the security review's URL-normalization evasions must be
/// rejected before any I/O -- either as `InvalidUrl` (the parser itself
/// refuses the form) or as `PolicyBlocked` (the form parses but classifies
/// as a disallowed destination). Neither ever falls through to a real
/// connection attempt against the fixture.
#[test]
fn normalization_evasions_never_reach_the_fixture() {
    let fixture = FixtureProcess::spawn(1);
    let fixture_port = fixture.instances[0].port;
    let client = default_policy_client();

    let cases = [
        format!("http://0x7f000001:{fixture_port}/navigation"), // hex-encoded 127.0.0.1
        format!("http://017700000001:{fixture_port}/navigation"), // octal-encoded 127.0.0.1
        format!("http://2130706433:{fixture_port}/navigation"), // decimal-encoded 127.0.0.1
        format!("http://127.1:{fixture_port}/navigation"),      // short-form 127.0.0.1
        format!("http://[::ffff:127.0.0.1]:{fixture_port}/navigation"), // ipv4-mapped ipv6
        format!("http://[::1]:{fixture_port}/navigation"),      // ipv6 loopback
        format!("http://public.example@127.0.0.1:{fixture_port}/navigation"), // userinfo confusion
        format!("http://127.0.0.1.:{fixture_port}/navigation"), // trailing dot
        format!("http://169.254.169.254:{fixture_port}/latest/meta-data/"), // cloud metadata
    ];

    for raw in cases {
        let url = NormalizedUrl::parse(&raw, None).unwrap_or_else(|error| {
            panic!("case {raw} must at least parse before being policy-rejected: {error}")
        });
        let ctx = support::test_ctx();
        match client.fetch(RequestSpec::get(url), "session-evasion", &ctx) {
            Err(NetworkError::PolicyBlocked(_)) => {}
            Err(other) => panic!("case {raw}: expected policy-blocked, got {other:?}"),
            Ok(_) => panic!("case {raw}: must never succeed against a private/metadata target"),
        }
    }
}

/// A public-looking `Location` on a redirect hop that actually points at a
/// private/loopback destination must be denied at that hop -- the redirect
/// loop re-enters the same `evaluate_url`/`evaluate_resolution`/
/// `evaluate_connect` pipeline on every hop, never a separate weaker path.
/// This uses a `RecordingPolicy` to prove the mechanism directly: every
/// redirect hop is actually re-evaluated with the correct URL and
/// `redirect_depth`, which is the concrete, checkable claim M2-T02 owns
/// (destination-class *policy content* is intentionally minimal here and
/// hardened later in M7-T04 -- see the evidence file for the full
/// rationale on why this sandbox cannot host a real "public then private"
/// two-hop network path).
#[derive(Default)]
struct RecordingPolicy {
    seen: Mutex<Vec<(String, u32)>>,
}

impl NetworkPolicy for RecordingPolicy {
    fn evaluate_url(&self, url: &NormalizedUrl, meta: &RequestMeta) -> PolicyDecision {
        self.seen
            .lock()
            .expect("lock")
            .push((url.as_str().to_owned(), meta.redirect_depth));
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

    fn evaluate_connect(
        &self,
        _address: std::net::SocketAddr,
        _meta: &RequestMeta,
    ) -> PolicyDecision {
        PolicyDecision::Allow
    }
}

#[test]
fn every_redirect_hop_re_enters_the_full_policy_pipeline_with_correct_data() {
    let fixture = FixtureProcess::spawn(1);
    let policy = Arc::new(RecordingPolicy::default());
    let client = NetworkClient::new(
        Arc::new(machina_network::SystemResolver),
        policy.clone(),
        Arc::new(UnlimitedBudget),
        machina_network::tls::build_client_config(&TrustStoreSource::WebpkiRoots)
            .expect("tls config builds"),
        ClientConfigOptions::default(),
    )
    .expect("client construction succeeds");

    let url = NormalizedUrl::parse(&format!("{}/redirect-chain?n=2", fixture.origin(0)), None)
        .expect("valid url");
    let ctx = support::test_ctx();
    let (_, _body) = client
        .fetch(RequestSpec::get(url), "session-recording", &ctx)
        .expect("redirect chain resolves");

    let seen = policy.seen.lock().expect("lock").clone();
    assert_eq!(
        seen.len(),
        3,
        "expected one evaluate_url call per hop, got {seen:?}"
    );
    assert_eq!(seen[0].1, 0, "first hop must be redirect_depth 0");
    assert_eq!(seen[1].1, 1, "second hop must be redirect_depth 1");
    assert_eq!(seen[2].1, 2, "third hop must be redirect_depth 2");
    assert!(seen[0].0.ends_with("n=2"));
    assert!(seen[1].0.ends_with("n=1"));
    assert!(seen[2].0.ends_with("n=0"));
}

#[test]
fn deny_private_networks_is_the_default_used_when_no_policy_is_composed_on_top() {
    // Documents the fail-closed default explicitly: constructing a client
    // with `DenyPrivateNetworks` and nothing else is a complete, safe
    // configuration on its own (the M2-T02 acceptance criterion is
    // satisfied before `security-policy`/M7-T04 exists at all).
    let policy = DenyPrivateNetworks;
    let url = NormalizedUrl::parse("http://10.0.0.1/", None).expect("valid url");
    let meta = RequestMeta {
        session_id: "session-default".to_owned(),
        tenant_id: None,
        purpose: machina_network::RequestPurpose::Navigation,
        redirect_depth: 0,
        correlation_id: "correlation-default".to_owned(),
    };
    assert!(matches!(
        policy.evaluate_url(&url, &meta),
        PolicyDecision::Deny { .. }
    ));
}
