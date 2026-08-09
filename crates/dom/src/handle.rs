//! Handle types: a process-unique document identity, an arena
//! index/generation pair, and kind-typed handles produced only via a
//! checked downcast on [`crate::document::Document`].
//!
//! Every handle is a small `Copy` value. Holding one after its node or
//! document is gone is always safe: it is inert data (two integers plus a
//! document id), never a pointer, so dereferencing it can only fail a
//! bounds/generation check — it can never alias freed memory or panic.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

/// Process-unique identity for one [`crate::document::Document`] (and
/// therefore one arena and one generation space).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DocumentId(u64);

impl DocumentId {
    pub(crate) fn next() -> Self {
        DocumentId(NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// The raw process-unique value. Exposed for logging/diagnostics only;
    /// callers must not assume any particular numbering scheme.
    pub fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "document#{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct NodeIndex(pub(crate) u32);

/// Widened to `u64` (not `u32`) per the M2-T05 security review, finding 1:
/// at realistic single-slot create/free churn a `u32` generation counter is
/// reachable within a multi-day automation session, and the failure mode on
/// wraparound would be silent handle aliasing (an old stale handle
/// resolving to an unrelated node) rather than a safe typed error. A `u64`
/// counter makes wraparound astronomically unreachable at the cost of 4
/// bytes per handle (20 -> 24 bytes), matching the standard fix used by
/// comparable generational-arena crates.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct Generation(pub(crate) u64);

/// A checked reference to one node inside one document's arena.
///
/// `NodeHandle` never aliases live memory: every dereference goes through
/// [`crate::document::Document::node`] (or an internal equivalent), which
/// checks the owning document, the arena bounds, and the generation before
/// returning any data. A stale, forged, or cross-document handle always
/// fails with a typed [`crate::error::DomError`] instead of panicking.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NodeHandle {
    pub(crate) document: DocumentId,
    pub(crate) index: NodeIndex,
    pub(crate) generation: Generation,
}

impl NodeHandle {
    /// The document this handle claims to belong to. Does not imply the
    /// handle is still valid in that document.
    pub fn document(self) -> DocumentId {
        self.document
    }
}

macro_rules! kind_handle {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        pub struct $name(pub(crate) NodeHandle);

        impl $name {
            /// The underlying untyped handle.
            pub fn node_handle(self) -> NodeHandle {
                self.0
            }

            /// The document this handle claims to belong to.
            pub fn document(self) -> DocumentId {
                self.0.document
            }
        }

        impl From<$name> for NodeHandle {
            fn from(value: $name) -> NodeHandle {
                value.0
            }
        }
    };
}

kind_handle!(
    /// A handle known (at the point it was produced) to reference an
    /// `Element` node. Produced only by
    /// [`crate::document::Document::create_element`] or
    /// [`crate::document::Document::as_element`].
    ElementHandle
);
kind_handle!(
    /// A handle known (at the point it was produced) to reference a `Text`
    /// node. Produced only by [`crate::document::Document::create_text`] or
    /// [`crate::document::Document::as_text`].
    TextHandle
);
kind_handle!(
    /// A handle known (at the point it was produced) to reference a
    /// `DocumentFragment` node. Produced only by
    /// [`crate::document::Document::create_document_fragment`] or
    /// [`crate::document::Document::as_document_fragment`].
    DocumentFragmentHandle
);
kind_handle!(
    /// A handle known (at the point it was produced) to reference a
    /// `DocumentType` node. Produced only by
    /// [`crate::document::Document::create_document_type`].
    DocumentTypeHandle
);
