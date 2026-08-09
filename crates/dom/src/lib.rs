//! Arena-backed DOM node storage: generational handles, mutation
//! invariants, wrapper-notification hooks, and document teardown.
//!
//! One [`Document`] owns one arena and one generation space; dropping or
//! closing a `Document` structurally releases every node it owns (no
//! `Rc`/`Arc`/raw pointers are ever used for tree structure), and every
//! handle dereference is checked, so a stale or cross-document
//! [`NodeHandle`] can never alias live memory or panic — it always fails
//! with a typed [`DomError`].
//!
//! This is a leaf, engine-internal data crate: no dependency on
//! `machina-native-core`, `machina-session`, `machina-command-bus`,
//! `machina-command-model`, or any protocol/control-plane crate, no
//! `unsafe`, and no `unwrap`/panic on any caller-reachable path. HTML
//! parsing, CSS selectors, shadow DOM, V8 wrapper objects, and semantic
//! layout are all out of scope here — see the M2-T05 design doc's
//! "Explicit non-goals" section.

mod arena;
mod attributes;
mod document;
mod error;
mod handle;
mod intern;
mod mutation;
mod node;
mod observer;

pub use document::{Document, DomMemoryUsage, Revision};
pub use error::DomError;
pub use handle::{
    DocumentFragmentHandle, DocumentId, DocumentTypeHandle, ElementHandle, NodeHandle, TextHandle,
};
pub use mutation::MAX_ANCESTOR_WALK;
pub use node::{Namespace, NodeKind, NodeRef};
pub use observer::{NodeChange, WrapperObserver};
