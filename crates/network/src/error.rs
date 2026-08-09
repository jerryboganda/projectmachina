//! Crate-local error type. `machina-network` intentionally does not depend
//! on `machina-command-model`; the caller (`native-core`) owns the mapping
//! from `NetworkError` to `CanonicalErrorCode` (see the M2-T02 design doc,
//! section 9). Every variant here corresponds 1:1 to a documented, typed
//! failure mode -- nothing here is a silently-downgraded success.

use std::fmt::{Display, Formatter};

/// A single policy-evaluation denial reason, surfaced verbatim from the
/// `NetworkPolicy` hook so callers/audit logs can tell *why* a destination
/// was rejected, not just that it was.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DenyReason {
    UnsupportedScheme,
    EmbeddedCredentials,
    InvalidHost,
    Loopback,
    LinkLocal,
    CloudMetadata,
    PrivateNetwork,
    UniqueLocalV6,
    CarrierGradeNat,
    ReservedRange,
    Multicast,
    Broadcast,
    UnspecifiedAddress,
    BlockedPort,
    MixedAnswerSet,
    RedirectDowngrade,
    PolicyRejected(String),
}

impl Display for DenyReason {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedScheme => formatter.write_str("unsupported scheme"),
            Self::EmbeddedCredentials => formatter.write_str("embedded credentials in URL"),
            Self::InvalidHost => formatter.write_str("invalid or ambiguous host"),
            Self::Loopback => formatter.write_str("loopback destination"),
            Self::LinkLocal => formatter.write_str("link-local destination"),
            Self::CloudMetadata => formatter.write_str("cloud metadata destination"),
            Self::PrivateNetwork => formatter.write_str("private (RFC1918) destination"),
            Self::UniqueLocalV6 => formatter.write_str("unique-local IPv6 destination"),
            Self::CarrierGradeNat => formatter.write_str("carrier-grade NAT destination"),
            Self::ReservedRange => formatter.write_str("reserved/special-use destination"),
            Self::Multicast => formatter.write_str("multicast destination"),
            Self::Broadcast => formatter.write_str("broadcast destination"),
            Self::UnspecifiedAddress => formatter.write_str("unspecified address"),
            Self::BlockedPort => formatter.write_str("blocked port"),
            Self::MixedAnswerSet => {
                formatter.write_str("DNS answer set contains at least one disallowed address")
            }
            Self::RedirectDowngrade => formatter.write_str("unsafe cross-scheme redirect"),
            Self::PolicyRejected(message) => formatter.write_str(message),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkError {
    /// URL failed to parse, used a non-network scheme, or otherwise cannot
    /// be normalized (WHATWG `url` crate rejected it or it is out of scope,
    /// e.g. `data:`/`blob:`/`file:`/`javascript:`).
    InvalidUrl(String),
    /// A `NetworkPolicy` call site denied the request.
    PolicyBlocked(DenyReason),
    /// DNS resolution failed outright (NXDOMAIN, resolver error, timeout).
    DnsResolutionFailed(String),
    /// TCP connect failed to the single pinned, policy-approved address.
    ConnectFailed(String),
    /// TLS handshake or certificate validation failed.
    TlsFailed(String),
    /// The wire protocol was malformed: bad status line, oversized/invalid
    /// headers, conflicting `Content-Length`/`Transfer-Encoding`, invalid
    /// chunk framing, CR/LF injection in a header value, etc.
    ProtocolError(String),
    /// `Content-Encoding` was absent/unsupported for a stacked value, or the
    /// declared encoding did not match the actual byte stream.
    DecompressionFailed(String),
    /// A streamed body exceeded an absolute size ceiling or compression
    /// ratio ceiling before completing.
    BodyTooLarge {
        limit: u64,
    },
    /// The caller-supplied `RequestBudget` rejected the request or a chunk.
    BudgetExceeded(String),
    /// Redirect chain exceeded `max_redirects`, contained a detected loop,
    /// or the `Location` header was missing/unparseable.
    TooManyRedirects,
    RedirectLoop,
    InvalidRedirect(String),
    /// `ctx.deadline` was reached during connect, handshake, header read, or
    /// a body chunk read.
    DeadlineExceeded,
    /// No body data arrived for longer than the configured idle-read
    /// timeout, distinct from the request's absolute deadline (defends
    /// against a connection that opens and then trickles nothing at all,
    /// or stalls between chunks, for up to the full remaining deadline).
    IdleReadTimeout,
    /// `ctx.cancellation` was observed cancelled during any I/O phase.
    Cancelled,
    /// An internal I/O error not covered by a more specific variant above.
    Io(String),
}

impl Display for NetworkError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl(message) => write!(formatter, "invalid url: {message}"),
            Self::PolicyBlocked(reason) => write!(formatter, "network policy blocked: {reason}"),
            Self::DnsResolutionFailed(message) => {
                write!(formatter, "dns resolution failed: {message}")
            }
            Self::ConnectFailed(message) => write!(formatter, "connect failed: {message}"),
            Self::TlsFailed(message) => write!(formatter, "tls failed: {message}"),
            Self::ProtocolError(message) => write!(formatter, "protocol error: {message}"),
            Self::DecompressionFailed(message) => {
                write!(formatter, "decompression failed: {message}")
            }
            Self::BodyTooLarge { limit } => {
                write!(formatter, "response body exceeded limit of {limit} bytes")
            }
            Self::BudgetExceeded(message) => write!(formatter, "budget exceeded: {message}"),
            Self::TooManyRedirects => formatter.write_str("too many redirects"),
            Self::RedirectLoop => formatter.write_str("redirect loop detected"),
            Self::InvalidRedirect(message) => write!(formatter, "invalid redirect: {message}"),
            Self::DeadlineExceeded => formatter.write_str("deadline exceeded"),
            Self::IdleReadTimeout => formatter.write_str("idle read timeout"),
            Self::Cancelled => formatter.write_str("cancelled"),
            Self::Io(message) => write!(formatter, "io error: {message}"),
        }
    }
}

impl std::error::Error for NetworkError {}
