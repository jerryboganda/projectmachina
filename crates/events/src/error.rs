//! Typed failure modes for every operation this crate exposes. No operation
//! in `crates/events` panics on caller-reachable input; every such condition
//! maps to one of these variants instead (matching `machina_dom::DomError`'s
//! own no-panic discipline).

use std::fmt;

use machina_dom::DomError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventError {
    /// The target handle does not currently resolve (stale, freed, wrong
    /// document, or the owning document has been closed).
    TargetNotFound,
    /// A listener-table operation was attempted with a handle that belongs
    /// to a different document than the one this registry was built for.
    WrongDocument,
    /// `perform_click`'s narrow interactability precheck failed: the target
    /// does not currently have an ancestor chain reaching the document root
    /// (see the crate-level docs' "attached, not fully interactable" note).
    NotInteractable,
    /// A `machina_dom` operation that this crate expected to succeed (the
    /// target was already validated) failed anyway — surfaced verbatim
    /// rather than silently mapped, since it usually signals a genuine
    /// concurrent/adversarial mutation the caller should see.
    Dom(DomError),
}

impl fmt::Display for EventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetNotFound => formatter.write_str("event target does not currently resolve"),
            Self::WrongDocument => {
                formatter.write_str("handle belongs to a different document than this registry")
            }
            Self::NotInteractable => {
                formatter.write_str("target is not attached to the document and cannot be clicked")
            }
            Self::Dom(err) => write!(formatter, "underlying dom error: {err}"),
        }
    }
}

impl std::error::Error for EventError {}

impl From<DomError> for EventError {
    fn from(err: DomError) -> Self {
        match err {
            DomError::WrongDocument => Self::WrongDocument,
            DomError::StaleHandle | DomError::DocumentClosed => Self::TargetNotFound,
            other => Self::Dom(other),
        }
    }
}
