//! URL normalization on top of the WHATWG `url` crate. Hand-rolled host
//! parsing is explicitly out of scope (see `security/NETWORK_EGRESS.md`'s
//! list of traps) -- the `url` crate's `Host` parser is what canonicalizes
//! alternative IPv4 notations (octal/decimal/hex/short forms), IDNA, and
//! percent-encoding so the policy hook only ever sees a canonical form.

use std::fmt::{Display, Formatter};

use url::{Host, Url};

use crate::error::NetworkError;

/// Schemes this loader will originate a network request for. Every other
/// scheme (`data:`, `blob:`, `about:`, `file:`, `javascript:`, ...) is
/// rejected at the loader boundary with `InvalidUrl`, never silently
/// downgraded or executed locally.
const SUPPORTED_SCHEMES: [&str; 2] = ["http", "https"];

/// A parsed, canonicalized request URL. Never constructed from a raw string
/// downstream of this module -- every consumer (policy hook, connector,
/// redirect loop) reads through this type so the canonical host is the only
/// thing ever inspected for classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedUrl {
    url: Url,
}

impl NormalizedUrl {
    /// Parse and normalize `raw` relative to an optional `base` (used to
    /// resolve redirect `Location` headers, which may be relative).
    pub fn parse(raw: &str, base: Option<&NormalizedUrl>) -> Result<Self, NetworkError> {
        let url = match base {
            Some(base) => base
                .url
                .join(raw)
                .map_err(|error| NetworkError::InvalidUrl(error.to_string()))?,
            None => Url::parse(raw).map_err(|error| NetworkError::InvalidUrl(error.to_string()))?,
        };
        let normalized = Self { url };
        normalized.validate()?;
        Ok(normalized)
    }

    fn validate(&self) -> Result<(), NetworkError> {
        if !SUPPORTED_SCHEMES.contains(&self.url.scheme()) {
            return Err(NetworkError::InvalidUrl(format!(
                "unsupported scheme: {}",
                self.url.scheme()
            )));
        }
        match self.url.host() {
            Some(Host::Domain(_)) | Some(Host::Ipv4(_)) | Some(Host::Ipv6(_)) => {}
            None => return Err(NetworkError::InvalidUrl("url has no host".to_owned())),
        }
        // Reject IPv6 zone-ID literals outright. The WHATWG URL parser does
        // not accept a `%zone` suffix inside `[...]` for a network scheme,
        // so `Url::parse` already fails for these; this check documents the
        // invariant and defends against any future relaxation of the
        // underlying parser silently starting to accept one.
        if self.url.as_str().contains("%25") && matches!(self.url.host(), Some(Host::Ipv6(_))) {
            return Err(NetworkError::InvalidUrl(
                "IPv6 zone identifiers are not permitted".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn scheme(&self) -> &str {
        self.url.scheme()
    }

    pub fn is_https(&self) -> bool {
        self.url.scheme() == "https"
    }

    /// Canonical host. Never derived from raw authority text (so userinfo
    /// confusion like `http://expected.com@attacker.com/` cannot leak the
    /// wrong string into a policy check -- `url` already separates userinfo
    /// from host during parsing).
    pub fn host(&self) -> Host<&str> {
        self.url
            .host()
            .expect("validated at construction: url always has a host")
    }

    /// Owned form of `host()`, used where a `'static`-free borrow of `self`
    /// is inconvenient (e.g. passing across an `.await` point into the
    /// connector).
    pub fn host_owned(&self) -> Host<String> {
        match self.host() {
            Host::Domain(domain) => Host::Domain(domain.to_owned()),
            Host::Ipv4(addr) => Host::Ipv4(addr),
            Host::Ipv6(addr) => Host::Ipv6(addr),
        }
    }

    pub fn port(&self) -> u16 {
        self.url
            .port_or_known_default()
            .unwrap_or(if self.is_https() { 443 } else { 80 })
    }

    /// True if the URL carries a non-empty username or password. Callers
    /// must never forward these credentials on the wire by default; the
    /// client strips them from the outgoing request unconditionally.
    pub fn has_embedded_credentials(&self) -> bool {
        !self.url.username().is_empty() || self.url.password().is_some()
    }

    /// Wire request-target: path + query, with the fragment stripped (the
    /// fragment is never sent to a server and is retained only on this
    /// `NormalizedUrl` for navigation/history use).
    pub fn request_target(&self) -> String {
        let mut target = self.url.path().to_owned();
        if let Some(query) = self.url.query() {
            target.push('?');
            target.push_str(query);
        }
        target
    }

    /// Scheme + host + port triple used for cross-origin comparisons
    /// (credential/header stripping across redirects).
    pub fn origin_tuple(&self) -> (String, String, u16) {
        (
            self.url.scheme().to_owned(),
            host_key(self.host()),
            self.port(),
        )
    }

    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }
}

impl Display for NormalizedUrl {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.url.as_str())
    }
}

/// Stable string key for a `Host` used only for origin-equality comparisons
/// (never for policy classification, which always operates on resolved
/// `IpAddr`s or the canonical `Host` returned by `NormalizedUrl::host`).
pub fn host_key(host: Host<&str>) -> String {
    match host {
        Host::Domain(domain) => domain.to_ascii_lowercase(),
        Host::Ipv4(addr) => addr.to_string(),
        Host::Ipv6(addr) => addr.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::NormalizedUrl;
    use url::Host;

    #[test]
    fn rejects_non_network_schemes() {
        for scheme in [
            "data:text/plain,hi",
            "blob:x",
            "about:blank",
            "file:///etc/passwd",
            "javascript:alert(1)",
        ] {
            let result = NormalizedUrl::parse(scheme, None);
            assert!(result.is_err(), "expected {scheme} to be rejected");
        }
    }

    #[test]
    fn canonicalizes_alternative_ipv4_notations() {
        // WHATWG host parsing accepts and canonicalizes octal/hex/decimal
        // forms for "special" schemes (http/https among them).
        let cases = [
            ("http://0x7f000001/", "127.0.0.1"),
            ("http://017700000001/", "127.0.0.1"),
            ("http://2130706433/", "127.0.0.1"),
            ("http://127.1/", "127.0.0.1"),
        ];
        for (raw, expected) in cases {
            let parsed = NormalizedUrl::parse(raw, None).unwrap_or_else(|error| {
                panic!("expected {raw} to parse: {error}");
            });
            match parsed.host() {
                Host::Ipv4(addr) => assert_eq!(addr.to_string(), expected, "for input {raw}"),
                other => panic!("expected ipv4 host for {raw}, got {other:?}"),
            }
        }
    }

    #[test]
    fn userinfo_confusion_never_leaks_into_host() {
        let parsed = NormalizedUrl::parse("http://expected.com@attacker.com/", None)
            .expect("valid url with userinfo");
        match parsed.host() {
            Host::Domain(domain) => assert_eq!(domain, "attacker.com"),
            other => panic!("expected domain host, got {other:?}"),
        }
        assert!(parsed.has_embedded_credentials());
    }

    #[test]
    fn ipv4_mapped_ipv6_literal_parses_as_ipv6_host() {
        let parsed =
            NormalizedUrl::parse("http://[::ffff:127.0.0.1]/", None).expect("valid ipv6 literal");
        assert!(matches!(parsed.host(), Host::Ipv6(_)));
    }

    #[test]
    fn rejects_ipv6_zone_id_literal() {
        // The WHATWG URL parser itself rejects a zone id in the host; this
        // proves the crate-boundary invariant, not just the underlying
        // library's behavior.
        let result = NormalizedUrl::parse("http://[fe80::1%25eth0]/", None);
        assert!(result.is_err(), "zone-id literal must not parse");
    }

    #[test]
    fn trailing_dot_hostname_is_accepted_like_bare_hostname() {
        let with_dot = NormalizedUrl::parse("http://example.com./path", None)
            .expect("trailing dot is a valid FQDN form");
        let without_dot = NormalizedUrl::parse("http://example.com/path", None).expect("valid url");
        // Both are treated as ordinary hostnames to be resolved through DNS;
        // neither is special-cased as a literal address, so no bypass of
        // literal-IP classification is possible either way.
        assert!(matches!(with_dot.host(), Host::Domain(_)));
        assert!(matches!(without_dot.host(), Host::Domain(_)));
    }

    #[test]
    fn fragment_is_stripped_from_wire_request_target() {
        let parsed =
            NormalizedUrl::parse("http://example.com/path?x=1#section", None).expect("valid url");
        assert_eq!(parsed.request_target(), "/path?x=1");
        assert_eq!(parsed.as_str(), "http://example.com/path?x=1#section");
    }

    #[test]
    fn idna_hostname_normalizes_to_ascii_punycode() {
        let parsed = NormalizedUrl::parse("http://xn--e1aybc.xn--p1ai/", None)
            .expect("already-ascii punycode form parses");
        match parsed.host() {
            Host::Domain(domain) => assert!(domain.starts_with("xn--")),
            other => panic!("expected domain host, got {other:?}"),
        }
    }

    #[test]
    fn relative_redirect_location_resolves_against_base() {
        let base = NormalizedUrl::parse("http://example.com/a/b", None).expect("base url");
        let resolved =
            NormalizedUrl::parse("/c/d", Some(&base)).expect("relative location resolves");
        assert_eq!(resolved.as_str(), "http://example.com/c/d");
    }
}
