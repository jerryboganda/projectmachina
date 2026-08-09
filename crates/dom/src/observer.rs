//! The single mutation-notification hook this crate exposes toward the V8
//! wrapper-object work (M2-T06/T07). Deliberately minimal and free of any
//! V8/binding type in its signature: this crate only reports "what changed
//! and when", never how a caller should react to it.

use crate::handle::{DocumentId, NodeHandle};

/// What kind of change a node just went through. `Inserted` and `Detached`
/// are distinct from freeing: a node can be `Detached` (still resolvable,
/// just orphaned) long before its slot is ever freed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeChange {
    Inserted,
    Detached,
    AttributesChanged,
    TextChanged,
    /// Fired once, on the *new* handle, when `adopt_node` finishes building
    /// the moved subtree's new root in the destination document. Because
    /// this crate's `adopt_node` frees and re-creates the subtree rather
    /// than relocating it in place, the new handle is never equal to
    /// `previous` — this variant exists specifically to give a future
    /// wrapper cache (M2-T06/T07) a correlation signal to re-key an
    /// existing wrapper instead of only observing an unrelated free+create
    /// pair (M2-T05 security review, finding 2b).
    Adopted {
        previous: NodeHandle,
    },
}

/// Notified after a mutation has already committed. An observer cannot fail
/// or veto a mutation — it is informational only.
///
/// Threading contract: a [`crate::document::Document`] is usable from one
/// thread/task at a time (mirrors V8 isolate confinement), so the trait
/// requires `Send` (to allow moving a `Document` between tasks) but not
/// `Sync` (this crate never calls the observer concurrently).
pub trait WrapperObserver: Send {
    /// A node's tree position or content changed. Never fires for a
    /// rejected (`Err`) mutation attempt.
    fn on_node_changed(&self, handle: NodeHandle, change: NodeChange);

    /// A node's arena slot was actually freed; `handle` is now permanently
    /// stale. Fires once per freed node, not on mere detachment.
    fn on_node_freed(&self, handle: NodeHandle);

    /// The document was torn down. Fires exactly once per document, even
    /// though every node it held was implicitly freed at the same time —
    /// callers must not expect a matching `on_node_freed` per node here.
    fn on_document_teardown(&self, document: DocumentId);
}
