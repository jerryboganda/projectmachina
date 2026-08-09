//! Typed failure modes for every DOM operation. No operation in this crate
//! panics on caller-reachable input (bad handle, bad tree shape, adversarial
//! depth/width); every such condition maps to one of these variants instead.

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomError {
    /// The handle's index/generation no longer names a live node (freed,
    /// reused for a different node, or otherwise never valid).
    StaleHandle,
    /// The handle names a node in a different document than the one it was
    /// used against.
    WrongDocument,
    /// The document has been torn down ([`crate::document::Document::close`]
    /// was called, explicitly or via `Drop`).
    DocumentClosed,
    /// The node exists but is not of the kind the operation requires (for
    /// example, calling an element-only accessor on a text node).
    WrongKind,
    /// A `reference`/`child`/`old_child` argument does not currently belong
    /// to the given parent.
    NotFound,
    /// The requested mutation would create a cycle, attach a node under a
    /// kind that cannot hold it, or violate a document node's at-most-one
    /// element/doctype-child constraint.
    HierarchyViolation,
    /// `adopt_node` was called with a source document that is the same as
    /// the destination document.
    SameDocument,
    /// A bounded tree-depth or ancestor-walk guard was hit. This is a fail
    /// closed response to a pathological/adversarial tree shape, not a
    /// normal error.
    DepthLimitExceeded,
    /// `destroy_node` was called on a node that still has a parent (only a
    /// currently-detached node may be reclaimed).
    NodeStillAttached,
    /// `destroy_node` was called on a node that still has children (only a
    /// childless node may be reclaimed; free the children first).
    NodeHasChildren,
    /// A name argument (element local name, doctype name) was empty.
    InvalidName,
}

impl fmt::Display for DomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleHandle => {
                formatter.write_str("handle refers to a freed or reused node slot")
            }
            Self::WrongDocument => {
                formatter.write_str("handle belongs to a different document than the one used")
            }
            Self::DocumentClosed => formatter.write_str("document has been torn down"),
            Self::WrongKind => formatter.write_str("node is not of the requested kind"),
            Self::NotFound => {
                formatter.write_str("referenced node is not currently a child of the given parent")
            }
            Self::HierarchyViolation => formatter
                .write_str("mutation would create a cycle or violate a tree-shape invariant"),
            Self::SameDocument => {
                formatter.write_str("adopt_node requires a source document different from self")
            }
            Self::DepthLimitExceeded => {
                formatter.write_str("tree depth or ancestor-walk bound exceeded")
            }
            Self::NodeStillAttached => {
                formatter.write_str("destroy_node requires a currently-detached node")
            }
            Self::NodeHasChildren => {
                formatter.write_str("destroy_node requires a node with no children")
            }
            Self::InvalidName => formatter.write_str("name argument must not be empty"),
        }
    }
}

impl std::error::Error for DomError {}
