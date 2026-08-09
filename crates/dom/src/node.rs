//! Node storage: the open `NodeKind` enum, the per-kind payload, tree edges
//! as handle fields (a doubly linked sibling list, not a `Vec` of children
//! per node), and a borrowed read view (`NodeRef`).

use std::mem;

use crate::attributes::AttributeMap;
use crate::document::Revision;
use crate::handle::NodeHandle;
use crate::intern::Atom;

/// The kind of a DOM node. Left open (non-exhaustive from the outside is
/// not needed yet, but new variants such as `ShadowRoot` are expected to be
/// added later without restructuring the arena) — see the M2-T05 design
/// doc's non-goals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Document,
    DocumentType,
    Element,
    Text,
    Comment,
    DocumentFragment,
}

/// The element namespace, needed for SVG/MathML foreign content (M2-T04).
/// `dom` does not implement foreign-content parsing rules itself; this is
/// just the tag so a namespace-aware element can exist in the tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
}

#[derive(Debug, Clone)]
pub(crate) enum NodePayload {
    Document,
    DocumentType {
        name: Box<str>,
        public_id: Box<str>,
        system_id: Box<str>,
    },
    Element {
        tag: Atom,
        namespace: Namespace,
        attributes: AttributeMap,
    },
    Text {
        data: Box<str>,
    },
    Comment {
        data: Box<str>,
    },
    DocumentFragment,
}

impl NodePayload {
    pub(crate) fn kind(&self) -> NodeKind {
        match self {
            Self::Document => NodeKind::Document,
            Self::DocumentType { .. } => NodeKind::DocumentType,
            Self::Element { .. } => NodeKind::Element,
            Self::Text { .. } => NodeKind::Text,
            Self::Comment { .. } => NodeKind::Comment,
            Self::DocumentFragment => NodeKind::DocumentFragment,
        }
    }
}

/// Content-only byte estimate for one node's payload (attribute values,
/// text/comment/doctype-name data). Does not include the fixed per-slot
/// overhead, which callers add separately via [`NODE_BASE_COST`].
pub(crate) fn payload_bytes(payload: &NodePayload) -> u64 {
    match payload {
        NodePayload::Document | NodePayload::DocumentFragment => 0,
        NodePayload::DocumentType {
            name,
            public_id,
            system_id,
        } => (name.len() + public_id.len() + system_id.len()) as u64,
        NodePayload::Element { attributes, .. } => attributes.bytes_estimate(),
        NodePayload::Text { data } | NodePayload::Comment { data } => data.len() as u64,
    }
}

/// Fixed accounted overhead for one arena slot, independent of its payload
/// content. Used to make [`crate::document::DomMemoryUsage`] track more than
/// just string bytes.
pub(crate) const NODE_BASE_COST: u64 = mem::size_of::<NodeData>() as u64;

#[derive(Debug, Clone, Default)]
pub(crate) struct NodeLinks {
    pub(crate) parent: Option<NodeHandle>,
    pub(crate) first_child: Option<NodeHandle>,
    pub(crate) last_child: Option<NodeHandle>,
    pub(crate) next_sibling: Option<NodeHandle>,
    pub(crate) previous_sibling: Option<NodeHandle>,
}

#[derive(Debug, Clone)]
pub(crate) struct NodeData {
    pub(crate) links: NodeLinks,
    pub(crate) payload: NodePayload,
    pub(crate) modified: Revision,
}

impl NodeData {
    pub(crate) fn kind(&self) -> NodeKind {
        self.payload.kind()
    }
}

/// A borrowed, read-only view of one resolved node.
#[derive(Clone, Copy, Debug)]
pub struct NodeRef<'a> {
    handle: NodeHandle,
    data: &'a NodeData,
}

impl<'a> NodeRef<'a> {
    pub(crate) fn new(handle: NodeHandle, data: &'a NodeData) -> Self {
        Self { handle, data }
    }

    pub fn handle(&self) -> NodeHandle {
        self.handle
    }

    pub fn kind(&self) -> NodeKind {
        self.data.kind()
    }

    pub fn parent(&self) -> Option<NodeHandle> {
        self.data.links.parent
    }

    pub fn first_child(&self) -> Option<NodeHandle> {
        self.data.links.first_child
    }

    pub fn last_child(&self) -> Option<NodeHandle> {
        self.data.links.last_child
    }

    pub fn next_sibling(&self) -> Option<NodeHandle> {
        self.data.links.next_sibling
    }

    pub fn previous_sibling(&self) -> Option<NodeHandle> {
        self.data.links.previous_sibling
    }

    pub fn modified(&self) -> Revision {
        self.data.modified
    }
}
