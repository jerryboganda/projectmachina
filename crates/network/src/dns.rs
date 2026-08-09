//! DNS resolution. `SystemResolver` performs explicit, inspectable
//! resolution (every A/AAAA answer is returned to the caller for
//! classification) rather than letting the HTTP client's connector resolve
//! opaquely -- this is what makes the mixed-answer-set and
//! resolve-then-connect-to-classified-IP defenses possible. A fully
//! pinned/cached rebinding-hardened resolver is `[DEFER -> M7-T04]`; this
//! crate provides only the call site (the `Resolver` trait) and a
//! correct-but-unpinned default implementation.

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;

use crate::error::NetworkError;

pub type ResolveFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, NetworkError>> + Send + 'a>>;

pub trait Resolver: Send + Sync {
    /// Resolve `host` to every candidate address (not just the first),
    /// without connecting. `port` is accepted for `SocketAddr` construction
    /// convenience only; the resolver must not filter or reorder by
    /// reachability.
    fn resolve<'a>(&'a self, host: &'a str, port: u16) -> ResolveFuture<'a>;
}

/// Default resolver: uses the OS/Tokio resolver via `tokio::net::lookup_host`
/// and returns every returned address. Explicit pre-connect classification
/// of the *entire* answer set (done by the caller, not this type) is what
/// closes the "opaque resolver" gap the security review calls out --
/// resolving is not the same as trusting the first answer.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemResolver;

impl Resolver for SystemResolver {
    fn resolve<'a>(&'a self, host: &'a str, port: u16) -> ResolveFuture<'a> {
        Box::pin(async move {
            let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
                .await
                .map_err(|error| NetworkError::DnsResolutionFailed(error.to_string()))?
                .collect();
            if addresses.is_empty() {
                return Err(NetworkError::DnsResolutionFailed(format!(
                    "no addresses returned for {host}"
                )));
            }
            Ok(addresses.into_iter().map(|socket| socket.ip()).collect())
        })
    }
}

/// A deterministic, injectable resolver for tests: returns a fixed answer
/// set (or a sequence of answer sets, to simulate rebinding between two
/// successive lookups of the same host).
#[cfg(any(test, feature = "test-support"))]
pub struct StaticResolver {
    answers: std::sync::Mutex<std::collections::VecDeque<Vec<IpAddr>>>,
    fallback: Vec<IpAddr>,
}

#[cfg(any(test, feature = "test-support"))]
impl StaticResolver {
    pub fn new(fallback: Vec<IpAddr>) -> Self {
        Self {
            answers: std::sync::Mutex::new(std::collections::VecDeque::new()),
            fallback,
        }
    }

    /// Queue a one-shot answer set; each call to `resolve` pops the next
    /// queued answer set until the queue is empty, then returns `fallback`.
    pub fn push_answer(&self, addresses: Vec<IpAddr>) {
        self.answers
            .lock()
            .expect("resolver lock poisoned")
            .push_back(addresses);
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Resolver for StaticResolver {
    fn resolve<'a>(&'a self, _host: &'a str, _port: u16) -> ResolveFuture<'a> {
        Box::pin(async move {
            let mut answers = self.answers.lock().expect("resolver lock poisoned");
            let addresses = answers.pop_front().unwrap_or_else(|| self.fallback.clone());
            if addresses.is_empty() {
                return Err(NetworkError::DnsResolutionFailed(
                    "static resolver has no configured answer".to_owned(),
                ));
            }
            Ok(addresses)
        })
    }
}
