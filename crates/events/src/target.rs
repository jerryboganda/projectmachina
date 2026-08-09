//! Listener storage and the DOM-lifecycle interop hooks.
//!
//! Listeners live in a side-table owned by this crate, not inside any
//! `machina_dom` node — symmetric with `machina_dom`'s own "stays agnostic"
//! ethos. The table is keyed by the full [`NodeHandle`] (index *and*
//! generation), not a bare arena index: `machina_dom::NodeHandle`'s index
//! field is `pub(crate)` to that crate (not exported), so a
//! generation-independent key is not available from outside it. This is a
//! documented, deliberate deviation from the original design note (which
//! asked for a `NodeIndex`-keyed table): keying by the full handle is
//! functionally equivalent here, because a live node's `NodeHandle` never
//! changes for the node's entire lifetime (from creation until its slot is
//! freed) — the generation only advances when a slot is *reused* for a
//! different node, at which point the old entry has already been removed by
//! [`EventTargetRegistry::handle_node_freed`]. No accessor request against
//! `crates/dom` was needed or made.

use std::collections::HashMap;
use std::rc::Rc;

use machina_dom::{Document, DocumentId, NodeChange, NodeHandle};

use crate::event::EventKind;
use crate::listener::{CallbackIdentity, EventListener, ListenerEntry, ListenerId};

/// Per-document listener table and focus state.
///
/// Not itself a `machina_dom::WrapperObserver` implementation: this crate
/// deliberately does not claim the single observer slot on `Document`
/// (other concerns — a future V8 wrapper GC bridge among them — will also
/// need it). See [`EventTargetRegistry::handle_node_changed`]'s doc comment
/// for the composing owner's required wiring shape.
pub struct EventTargetRegistry {
    document: DocumentId,
    by_node: HashMap<NodeHandle, Vec<ListenerEntry>>,
    active_element: Option<NodeHandle>,
    next_listener_id: u64,
}

impl EventTargetRegistry {
    pub fn new(document: DocumentId) -> Self {
        Self {
            document,
            by_node: HashMap::new(),
            active_element: None,
            next_listener_id: 1,
        }
    }

    pub fn for_document(document: &Document) -> Self {
        Self::new(document.id())
    }

    pub fn document(&self) -> DocumentId {
        self.document
    }

    pub fn active_element(&self) -> Option<NodeHandle> {
        self.active_element
    }

    pub(crate) fn set_active_element(&mut self, handle: Option<NodeHandle>) {
        self.active_element = handle;
    }

    fn check_document(&self, handle: NodeHandle) -> Result<(), crate::error::EventError> {
        if handle.document() != self.document {
            return Err(crate::error::EventError::WrongDocument);
        }
        Ok(())
    }

    /// Idempotent per spec: re-adding a listener already registered with the
    /// same `(event_kind, capture, identity)` on this node is a no-op that
    /// returns the existing [`ListenerId`] rather than creating a duplicate
    /// entry or moving it in registration order.
    #[allow(clippy::too_many_arguments)]
    pub fn add_event_listener(
        &mut self,
        target: NodeHandle,
        event_kind: EventKind,
        capture: bool,
        once: bool,
        passive: bool,
        identity: CallbackIdentity,
        callback: Rc<dyn EventListener>,
    ) -> Result<ListenerId, crate::error::EventError> {
        self.check_document(target)?;
        let entries = self.by_node.entry(target).or_default();
        if let Some(existing) = entries.iter().find(|entry| {
            entry.event_kind == event_kind && entry.capture == capture && entry.identity == identity
        }) {
            return Ok(existing.id);
        }
        let id = ListenerId(self.next_listener_id);
        self.next_listener_id += 1;
        entries.push(ListenerEntry {
            id,
            event_kind,
            capture,
            once,
            passive,
            identity,
            callback,
        });
        Ok(id)
    }

    /// Removes the listener matching `(event_kind, capture, identity)` on
    /// `target`, if any. Returns `Ok(true)` if a listener was removed.
    pub fn remove_event_listener(
        &mut self,
        target: NodeHandle,
        event_kind: EventKind,
        capture: bool,
        identity: CallbackIdentity,
    ) -> Result<bool, crate::error::EventError> {
        self.check_document(target)?;
        let Some(entries) = self.by_node.get_mut(&target) else {
            return Ok(false);
        };
        let before = entries.len();
        entries.retain(|entry| {
            !(entry.event_kind == event_kind
                && entry.capture == capture
                && entry.identity == identity)
        });
        let removed = entries.len() != before;
        if entries.is_empty() {
            self.by_node.remove(&target);
        }
        Ok(removed)
    }

    /// Removes a listener by the [`ListenerId`] returned from
    /// [`EventTargetRegistry::add_event_listener`]. Used internally to
    /// enforce `once` semantics, and exposed for direct removal without
    /// reconstructing the identity triple.
    pub fn remove_event_listener_by_id(
        &mut self,
        target: NodeHandle,
        id: ListenerId,
    ) -> Result<bool, crate::error::EventError> {
        self.check_document(target)?;
        let Some(entries) = self.by_node.get_mut(&target) else {
            return Ok(false);
        };
        let before = entries.len();
        entries.retain(|entry| entry.id != id);
        let removed = entries.len() != before;
        if entries.is_empty() {
            self.by_node.remove(&target);
        }
        Ok(removed)
    }

    /// Number of listeners currently registered on `target`, across all
    /// event kinds and capture flags. Diagnostic/test helper.
    pub fn listener_count(&self, target: NodeHandle) -> usize {
        self.by_node.get(&target).map_or(0, Vec::len)
    }

    /// A registration-order snapshot (cheap `Rc` clones, not a borrow of
    /// `self`) of `target`'s listeners matching `event_kind` and the given
    /// capture-phase filter. `want_capture = None` matches both (used for
    /// the at-target phase, which does not distinguish capture/bubble).
    pub(crate) fn snapshot_listeners(
        &self,
        target: NodeHandle,
        event_kind: EventKind,
        want_capture: Option<bool>,
    ) -> Vec<ListenerEntry> {
        self.by_node
            .get(&target)
            .into_iter()
            .flatten()
            .filter(|entry| {
                entry.event_kind == event_kind
                    && want_capture.is_none_or(|want| entry.capture == want)
            })
            .cloned()
            .collect()
    }

    /// Forwards a `machina_dom::WrapperObserver::on_node_changed`
    /// notification. Only `NodeChange::Detached` on the currently active
    /// element does anything (fires `blur`+`focusout`, then clears focus);
    /// every other change is ignored.
    ///
    /// **Integration note (why this takes `&mut Document` and cannot be the
    /// literal `WrapperObserver::on_node_changed` method body):**
    /// `WrapperObserver::on_node_changed(&self, ...)` is invoked by
    /// `machina_dom::Document` from *inside* an already-in-progress `&mut
    /// self` mutation call, so a `&mut Document` is not available at that
    /// call site (and re-entering the same document mid-mutation would be
    /// unsound even if it were). The composing owner's `WrapperObserver`
    /// impl must therefore buffer `(handle, change)` pairs during the raw
    /// callback (a cheap `Vec`/`RefCell<Vec<_>>` push, exactly the pattern
    /// `crates/dom`'s own test-only `RecordingObserver` already uses), then
    /// — once the top-level `Document` mutation call has returned and
    /// `&mut Document` is available again — drain the buffer and call this
    /// method once per entry.
    pub fn handle_node_changed(
        &mut self,
        document: &mut Document,
        handle: NodeHandle,
        change: NodeChange,
    ) -> Result<(), crate::error::EventError> {
        if change != NodeChange::Detached {
            return Ok(());
        }
        if self.active_element != Some(handle) {
            return Ok(());
        }
        if document.node(handle).is_err() {
            // Already gone by the time we got here; `handle_node_freed`
            // (belt-and-suspenders) will clear it silently.
            return Ok(());
        }
        self.active_element = None;
        crate::dispatch::dispatch_event(
            document,
            self,
            handle,
            EventKind::Blur,
            crate::event::EventInit::for_kind(EventKind::Blur),
        )?;
        crate::dispatch::dispatch_event(
            document,
            self,
            handle,
            EventKind::FocusOut,
            crate::event::EventInit::for_kind(EventKind::FocusOut),
        )?;
        Ok(())
    }

    /// Forwards a `machina_dom::WrapperObserver::on_node_freed`
    /// notification: drops every listener registered on `handle` (running
    /// their `Drop` impls — the GC-safety seam a V8-backed listener needs to
    /// release its persistent handle), and clears focus silently (no
    /// events — the handle is already permanently stale, so dispatch is
    /// impossible) if it was the active element.
    ///
    /// Unlike [`EventTargetRegistry::handle_node_changed`], this does not
    /// need `&mut Document` (it fires no events), so — unlike that method —
    /// it *can* be called directly from inside the raw
    /// `on_node_freed(&self, ...)` callback if the composing owner wraps
    /// this registry in interior mutability (for example
    /// `RefCell<EventTargetRegistry>`) for that purpose; buffering is only
    /// required for the `Document`-needing hook.
    pub fn handle_node_freed(&mut self, handle: NodeHandle) {
        self.by_node.remove(&handle);
        if self.active_element == Some(handle) {
            self.active_element = None;
        }
    }

    /// Forwards a `machina_dom::WrapperObserver::on_document_teardown`
    /// notification: drops every listener across every node and clears
    /// focus in one call, firing **no** synthetic events (mirrors
    /// `WrapperObserver`'s "fires once, not per-node" contract — this is
    /// the bulk-teardown counterpart to [`EventTargetRegistry::handle_node_freed`],
    /// not a loop over it). Like `handle_node_freed`, needs no `&mut
    /// Document` and is safe to call directly from the raw callback.
    pub fn handle_document_teardown(&mut self, document: DocumentId) {
        debug_assert_eq!(document, self.document);
        self.by_node.clear();
        self.active_element = None;
    }
}
