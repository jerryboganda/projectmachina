//! Typed failure modes. Per the design's pause/resume contract (§6):
//! malformed HTML *never* produces an `Err` here, only `Diagnostic`s
//! collected on the builder — `Err` is reserved for genuine
//! misuse-of-the-API or internal-invariant violations, and poisons the
//! instance (the caller must discard it; there is no "recover from the
//! recovery algorithm" retry, which would risk an infinite loop).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeBuilderError {
    /// `feed`/`finish` was called while the builder is paused at a script
    /// checkpoint. Call `resume_after_script` first.
    AlreadyPaused,
    /// `resume_after_script` was called while the builder is not paused.
    NotPaused,
    /// `feed`/`finish`/`resume_after_script` was called on an instance that
    /// already poisoned itself after a previous `Internal` error.
    Poisoned,
    /// A DOM call the tree builder believed valid returned an unexpected
    /// `DomError`, or an internal invariant (bounded loop, stack
    /// non-emptiness the algorithm assumed) did not hold. The instance is
    /// poisoned after this is returned — see module docs.
    Internal(String),
}

impl fmt::Display for TreeBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyPaused => {
                f.write_str("feed/finish called while paused at a script checkpoint")
            }
            Self::NotPaused => f.write_str("resume_after_script called while not paused"),
            Self::Poisoned => f.write_str("tree builder instance is poisoned; discard it"),
            Self::Internal(detail) => {
                write!(f, "internal tree-builder invariant violation: {detail}")
            }
        }
    }
}

impl std::error::Error for TreeBuilderError {}
