//! Typed failure modes for every extraction operation in this crate. No
//! path reachable from live DOM content panics (`unwrap`/`expect`) here —
//! every such condition maps to one of these variants instead, mirroring
//! `machina_dom::DomError` and `machina_selectors::QueryError`'s own
//! posture.

use std::fmt;

use machina_dom::DomError;
use machina_selectors::QueryError;

use crate::limits::LimitKind;

/// Canonical error type for every extraction entry point in this crate.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SemanticError {
    /// A DOM-layer failure (stale/cross-document/closed handle, wrong node
    /// kind, ...) surfaced while walking the document.
    DomError(DomError),
    /// A `machina-selectors` query failure surfaced while locating
    /// headings/links/forms/interactive-element candidates. In practice this
    /// crate only ever compiles its own fixed, hand-written selector text
    /// (never caller-supplied), so this variant should only ever be reached
    /// if that fixed text itself were malformed — which would be a bug in
    /// this crate, not a caller-input problem. Surfaced as a typed error
    /// rather than `unwrap`/`expect` on the compile call regardless, per
    /// this crate's own "no panic on any DOM-content-reachable path" bar.
    QueryError(QueryError),
    /// A bounded-walk guard (see [`LimitKind`]) was hit; the operation
    /// failed closed rather than doing unbounded work. This is the
    /// defensive-backstop path (a corrupted/cyclic tree state that should be
    /// structurally impossible per `machina_dom`'s own invariants) — a
    /// legitimately large real document instead hits the normal
    /// byte/item-count truncation path (see `SemanticIndex::truncated` /
    /// `MarkdownDocument::truncated`), which is not an error.
    TooComplex { limit: LimitKind },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DomError(inner) => write!(formatter, "dom error: {inner}"),
            Self::QueryError(inner) => write!(formatter, "query error: {inner}"),
            Self::TooComplex { limit } => write!(formatter, "too complex: {limit} bound exceeded"),
        }
    }
}

impl std::error::Error for SemanticError {}

impl From<DomError> for SemanticError {
    fn from(value: DomError) -> Self {
        SemanticError::DomError(value)
    }
}

impl From<QueryError> for SemanticError {
    fn from(value: QueryError) -> Self {
        SemanticError::QueryError(value)
    }
}
