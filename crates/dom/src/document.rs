//! `Document`: one arena, one generation space, one teardown unit. Owns
//! construction, read accessors, revision tracking, memory reporting, the
//! wrapper-observer slot, and explicit/implicit teardown. Mutation methods
//! (`insert_before`, `create_element`, `adopt_node`, ...) live in
//! [`crate::mutation`] as additional `impl Document` blocks in the same
//! crate.

use crate::arena::NodeArena;
use crate::error::DomError;
use crate::handle::{DocumentFragmentHandle, DocumentId, ElementHandle, NodeHandle, TextHandle};
use crate::intern::StringInterner;
use crate::node::{NodeData, NodeKind, NodePayload, NodeRef};
use crate::observer::{NodeChange, WrapperObserver};

/// A monotonic per-document counter, bumped once for every successful
/// mutation call that actually changes something. Deliberately coarse
/// (structural and content changes share one counter): callers that only
/// need "did anything change since I last looked" (for example a future
/// query-cache layer) can key off this without this crate having to guess
/// at a finer split.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Revision(pub(crate) u64);

impl Revision {
    pub fn value(self) -> u64 {
        self.0
    }
}

/// A point-in-time report of what one document has allocated. This crate
/// only *reports* usage; enforcing a budget against it is the owning
/// layer's job (`crates/session`'s `ResourceBudget`/`ResourceUsage`, per the
/// M2-T05 design doc).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomMemoryUsage {
    pub node_count: u64,
    pub bytes_estimate: u64,
}

pub struct Document {
    pub(crate) id: DocumentId,
    pub(crate) arena: NodeArena,
    pub(crate) interner: StringInterner,
    pub(crate) revision: Revision,
    pub(crate) observer: Option<Box<dyn WrapperObserver>>,
    pub(crate) torn_down: bool,
    pub(crate) bytes_estimate: u64,
    pub(crate) root: NodeHandle,
}

impl Document {
    /// Creates a new, empty document: one arena holding a single
    /// `NodeKind::Document` root node with no children.
    pub fn new() -> Self {
        let id = DocumentId::next();
        let mut arena = NodeArena::new();
        let revision = Revision(0);
        let root_data = NodeData {
            links: Default::default(),
            payload: NodePayload::Document,
            modified: revision,
        };
        let root = arena.insert(id, root_data);
        Document {
            id,
            arena,
            interner: StringInterner::default(),
            revision,
            observer: None,
            torn_down: false,
            bytes_estimate: crate::node::NODE_BASE_COST,
            root,
        }
    }

    /// This document's process-unique identity.
    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// The handle to this document's single `NodeKind::Document` root node.
    pub fn root(&self) -> NodeHandle {
        self.root
    }

    /// `true` once [`Document::close`] has run (explicitly or via `Drop`).
    pub fn is_closed(&self) -> bool {
        self.torn_down
    }

    /// The single choke point every dereference goes through: checks
    /// wrong-document, then document-closed, then stale-generation/freed
    /// slot (via the arena). Never panics on a bad handle.
    pub(crate) fn resolve(&self, handle: NodeHandle) -> Result<&NodeData, DomError> {
        if handle.document != self.id {
            return Err(DomError::WrongDocument);
        }
        if self.torn_down {
            return Err(DomError::DocumentClosed);
        }
        self.arena.resolve(handle)
    }

    pub(crate) fn resolve_mut(&mut self, handle: NodeHandle) -> Result<&mut NodeData, DomError> {
        if handle.document != self.id {
            return Err(DomError::WrongDocument);
        }
        if self.torn_down {
            return Err(DomError::DocumentClosed);
        }
        self.arena.resolve_mut(handle)
    }

    pub(crate) fn bump_revision(&mut self) {
        self.revision = Revision(self.revision.0.wrapping_add(1));
    }

    pub(crate) fn touch(&mut self, handle: NodeHandle) -> Result<(), DomError> {
        let revision = self.revision;
        let data = self.resolve_mut(handle)?;
        data.modified = revision;
        Ok(())
    }

    pub(crate) fn notify_changed(&self, handle: NodeHandle, change: NodeChange) {
        if let Some(observer) = &self.observer {
            observer.on_node_changed(handle, change);
        }
    }

    pub(crate) fn adjust_bytes(&mut self, removed: u64, added: u64) {
        self.bytes_estimate = self
            .bytes_estimate
            .saturating_sub(removed)
            .saturating_add(added);
    }

    /// Resolves a handle into a borrowed read view. Fails safely
    /// ([`DomError`], never a panic) for a stale, cross-document, or
    /// post-close handle.
    pub fn node(&self, handle: NodeHandle) -> Result<NodeRef<'_>, DomError> {
        self.resolve(handle).map(|data| NodeRef::new(handle, data))
    }

    /// Checked downcast: succeeds only if `handle` currently resolves to an
    /// `Element` node.
    pub fn as_element(&self, handle: NodeHandle) -> Result<ElementHandle, DomError> {
        let data = self.resolve(handle)?;
        match data.kind() {
            NodeKind::Element => Ok(ElementHandle(handle)),
            _ => Err(DomError::WrongKind),
        }
    }

    /// Checked downcast: succeeds only if `handle` currently resolves to a
    /// `Text` node.
    pub fn as_text(&self, handle: NodeHandle) -> Result<TextHandle, DomError> {
        let data = self.resolve(handle)?;
        match data.kind() {
            NodeKind::Text => Ok(TextHandle(handle)),
            _ => Err(DomError::WrongKind),
        }
    }

    /// Checked downcast: succeeds only if `handle` currently resolves to a
    /// `DocumentFragment` node.
    pub fn as_document_fragment(
        &self,
        handle: NodeHandle,
    ) -> Result<DocumentFragmentHandle, DomError> {
        let data = self.resolve(handle)?;
        match data.kind() {
            NodeKind::DocumentFragment => Ok(DocumentFragmentHandle(handle)),
            _ => Err(DomError::WrongKind),
        }
    }

    pub fn tag_name(&self, element: ElementHandle) -> Result<&str, DomError> {
        let data = self.resolve(element.node_handle())?;
        match &data.payload {
            NodePayload::Element { tag, .. } => Ok(self.interner.resolve(*tag)),
            _ => Err(DomError::WrongKind),
        }
    }

    pub fn attribute(&self, element: ElementHandle, name: &str) -> Result<Option<&str>, DomError> {
        let data = self.resolve(element.node_handle())?;
        let attributes = match &data.payload {
            NodePayload::Element { attributes, .. } => attributes,
            _ => return Err(DomError::WrongKind),
        };
        let atom = match self.interner.find(name) {
            Some(atom) => atom,
            None => return Ok(None),
        };
        Ok(attributes.get(atom))
    }

    /// Text content of a `Text` or `Comment` node.
    pub fn text_data(&self, handle: NodeHandle) -> Result<&str, DomError> {
        let data = self.resolve(handle)?;
        match &data.payload {
            NodePayload::Text { data } | NodePayload::Comment { data } => Ok(data),
            _ => Err(DomError::WrongKind),
        }
    }

    /// The current children of `parent`, in tree order.
    pub fn children(&self, parent: NodeHandle) -> Result<Vec<NodeHandle>, DomError> {
        crate::mutation::collect_children(self, parent)
    }

    /// The current revision counter. Strictly increases on every successful
    /// mutation that changes something; unchanged on `Err` or a no-op call.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// `bytes_estimate` includes both per-node payload/base cost and the
    /// dynamic (per-document) interner's custom tag/attribute-name storage
    /// — so adversarial custom-tag growth is accounted the same as any
    /// other page content, not silently excluded.
    pub fn memory_usage(&self) -> DomMemoryUsage {
        DomMemoryUsage {
            node_count: self.arena.live_count(),
            bytes_estimate: self.bytes_estimate + self.interner.bytes_estimate(),
        }
    }

    /// Installs (or clears, with `None`) the single wrapper-observer slot.
    pub fn set_wrapper_observer(&mut self, observer: Option<Box<dyn WrapperObserver>>) {
        self.observer = observer;
    }

    /// Idempotent explicit teardown: notifies `on_document_teardown` once
    /// (only the first call fires it), then structurally releases every
    /// node this document owns by replacing the arena — dropping the one
    /// `Vec<Slot>` synchronously frees every owned `String`/`Box<str>`, not
    /// on a later GC pass. After this call every handle into this document,
    /// including ones issued before the call, resolves to
    /// [`DomError::DocumentClosed`].
    pub fn close(&mut self) {
        if self.torn_down {
            return;
        }
        self.torn_down = true;
        if let Some(observer) = &self.observer {
            observer.on_document_teardown(self.id);
        }
        self.arena = NodeArena::new();
        self.bytes_estimate = 0;
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        // Scope-drop without an explicit `close()` call must still tear down
        // correctly and still fire `on_document_teardown` exactly once.
        self.close();
    }
}
