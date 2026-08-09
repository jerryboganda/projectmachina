//! Native URL, DNS, TLS and HTTP streaming loader (M2-T02).
//!
//! `machina-network` is the first crate in this workspace to own an async
//! runtime. It is deliberately self-contained: it does not depend on
//! `machina-session` or `machina-security-policy` (those compose on top of
//! the traits this crate defines -- `RequestBudget`, `NetworkPolicy`), and
//! it exposes a synchronous, blocking-callable public API today because the
//! rest of the engine is synchronous (`crates/event-loop`, M2-T08, depends
//! on this crate, not the reverse). See
//! `.agent-state/design/M2-T02-http-loader-design.md` and
//! `.agent-state/design/M2-T02-security-review.md` for the full design and
//! threat model this crate implements against.

pub mod address_class;
pub mod body;
pub mod budget;
pub mod client;
pub mod connector;
pub mod dns;
pub mod error;
pub mod policy;
pub mod ports;
pub mod request;
pub mod response;
pub mod runtime;
pub mod tls;
pub mod url;

pub use address_class::{classify, DestinationClass};
pub use body::{ResponseBody, ResponseLimits};
pub use budget::{RequestBudget, UnlimitedBudget};
pub use client::{ClientConfigOptions, NetworkClient};
pub use dns::{Resolver, SystemResolver};
pub use error::{DenyReason, NetworkError};
pub use policy::{DenyPrivateNetworks, NetworkPolicy, PolicyDecision, RequestMeta, RequestPurpose};
pub use request::RequestSpec;
pub use response::{RedirectHop, ResponseHead};
pub use runtime::NetworkRuntime;
pub use tls::TrustStoreSource;
pub use url::NormalizedUrl;

#[cfg(any(test, feature = "test-support"))]
pub use dns::StaticResolver;
