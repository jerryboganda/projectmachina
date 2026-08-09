//! Tree mutation: `insert_before`/`append_child`/`remove_child`/
//! `replace_child`/`adopt_node`/`clone_node`, node creation, attribute and
//! text mutation, and single-node reclamation (`destroy_node`).
//!
//! Every mutating method here is two-phase: a read-only validation pass
//! (against `self`, never touching any field) followed by a commit pass.
//! Commit-phase steps always re-resolve live links at the start of each
//! micro-step rather than acting on validate-phase-cached positions (see the
//! M2-T05 security review, finding 3a) — this is what makes self-aliased
//! arguments (`replace_child(p, x, x)`, `insert_before(p, x, x)`) safe
//! no-ops instead of silently corrupting the sibling list.

use crate::attributes::AttributeMap;
use crate::document::Document;
use crate::error::DomError;
use crate::handle::{
    DocumentFragmentHandle, DocumentTypeHandle, ElementHandle, NodeHandle, TextHandle,
};
use crate::intern::Atom;
use crate::node::{
    payload_bytes, Namespace, NodeData, NodeKind, NodeLinks, NodePayload, NODE_BASE_COST,
};
use crate::observer::NodeChange;

/// Bound on ancestor-walk (cycle detection) and on subtree depth for
/// `clone_node`/`adopt_node`. Traversal in this crate is always iterative
/// (an explicit heap-allocated stack, never native recursion), so this bound
/// does not need to protect the call stack — it only needs to fail closed
/// against unbounded work from a pathological or adversarial tree shape.
/// 100,000 is deliberately generous so legitimate deep documents (page
/// builders, minified framework output — five-figure depths are not
/// unheard of, per the M2-T05 security review finding 5a) are never
/// rejected by ordinary, non-cyclic inserts.
pub const MAX_ANCESTOR_WALK: usize = 100_000;

/// Purely defensive bound on a single node's sibling-list walk. Under
/// correct invariants a sibling chain cannot cycle, so this should never
/// trigger; it exists only so a hypothetical corrupted linked list fails
/// closed (`DepthLimitExceeded`) instead of looping forever.
const MAX_SIBLING_SCAN: usize = 10_000_000;

impl Document {
    // ---- node creation -----------------------------------------------

    /// Sugar for `create_element_ns(Namespace::Html, tag)`.
    pub fn create_element(&mut self, tag: &str) -> Result<ElementHandle, DomError> {
        self.create_element_ns(Namespace::Html, tag)
    }

    pub fn create_element_ns(
        &mut self,
        namespace: Namespace,
        local_name: &str,
    ) -> Result<ElementHandle, DomError> {
        if local_name.trim().is_empty() {
            return Err(DomError::InvalidName);
        }
        self.bump_revision();
        let tag = self.interner.intern(local_name);
        let payload = NodePayload::Element {
            tag,
            namespace,
            attributes: AttributeMap::new(),
        };
        let handle = self.insert_node(payload);
        Ok(ElementHandle(handle))
    }

    pub fn element_namespace(&self, element: ElementHandle) -> Result<Namespace, DomError> {
        let data = self.resolve(element.node_handle())?;
        match &data.payload {
            NodePayload::Element { namespace, .. } => Ok(*namespace),
            _ => Err(DomError::WrongKind),
        }
    }

    pub fn create_text(&mut self, data: &str) -> TextHandle {
        self.bump_revision();
        let handle = self.insert_node(NodePayload::Text { data: data.into() });
        TextHandle(handle)
    }

    /// Comments do not have a dedicated kind-typed handle in this design
    /// (only `Element`/`Text`/`DocumentFragment` do); callers use the plain
    /// `NodeHandle` with [`Document::text_data`]/[`Document::set_text_data`]
    /// (both accept `Text` or `Comment`).
    pub fn create_comment(&mut self, data: &str) -> NodeHandle {
        self.bump_revision();
        self.insert_node(NodePayload::Comment { data: data.into() })
    }

    pub fn create_document_fragment(&mut self) -> DocumentFragmentHandle {
        self.bump_revision();
        let handle = self.insert_node(NodePayload::DocumentFragment);
        DocumentFragmentHandle(handle)
    }

    pub fn create_document_type(
        &mut self,
        name: &str,
        public_id: &str,
        system_id: &str,
    ) -> Result<DocumentTypeHandle, DomError> {
        if name.trim().is_empty() {
            return Err(DomError::InvalidName);
        }
        self.bump_revision();
        let handle = self.insert_node(NodePayload::DocumentType {
            name: name.into(),
            public_id: public_id.into(),
            system_id: system_id.into(),
        });
        Ok(DocumentTypeHandle(handle))
    }

    /// Allocates a fresh, detached (no parent, no children) node in this
    /// document's arena and accounts its bytes. Does not bump the revision
    /// itself — callers bump once per public creation call.
    fn insert_node(&mut self, payload: NodePayload) -> NodeHandle {
        let bytes = payload_bytes(&payload);
        let revision = self.revision;
        let data = NodeData {
            links: NodeLinks::default(),
            payload,
            modified: revision,
        };
        let handle = self.arena.insert(self.id, data);
        self.adjust_bytes(0, NODE_BASE_COST + bytes);
        handle
    }

    // ---- attribute / text mutation ------------------------------------

    pub fn set_attribute(
        &mut self,
        element: ElementHandle,
        name: &str,
        value: &str,
    ) -> Result<(), DomError> {
        let handle = element.node_handle();
        // Ensure it really is an element before we intern anything or
        // touch the revision counter.
        match self.resolve(handle)?.kind() {
            NodeKind::Element => {}
            _ => return Err(DomError::WrongKind),
        }
        let atom = self.interner.intern(name);
        let boxed: Box<str> = value.into();
        let new_len = boxed.len() as u64;
        let data = self.resolve_mut(handle)?;
        let attributes = match &mut data.payload {
            NodePayload::Element { attributes, .. } => attributes,
            _ => return Err(DomError::WrongKind),
        };
        let previous_len = attributes.set(atom, boxed);
        self.adjust_bytes(previous_len.unwrap_or(0), new_len);
        self.bump_revision();
        self.touch(handle)?;
        self.notify_changed(handle, NodeChange::AttributesChanged);
        Ok(())
    }

    /// Returns `Ok(true)` if the attribute existed and was removed,
    /// `Ok(false)` (no revision bump, no notification — nothing changed)
    /// if it was already absent.
    pub fn remove_attribute(
        &mut self,
        element: ElementHandle,
        name: &str,
    ) -> Result<bool, DomError> {
        let handle = element.node_handle();
        let atom = match self.interner.find(name) {
            Some(atom) => atom,
            None => {
                // Never interned in this document at all; ensure the
                // element handle is at least valid before reporting "no-op".
                match self.resolve(handle)?.kind() {
                    NodeKind::Element => return Ok(false),
                    _ => return Err(DomError::WrongKind),
                }
            }
        };
        let data = self.resolve_mut(handle)?;
        let attributes = match &mut data.payload {
            NodePayload::Element { attributes, .. } => attributes,
            _ => return Err(DomError::WrongKind),
        };
        match attributes.remove(atom) {
            Some(removed_len) => {
                self.adjust_bytes(removed_len, 0);
                self.bump_revision();
                self.touch(handle)?;
                self.notify_changed(handle, NodeChange::AttributesChanged);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Sets the text content of a `Text` or `Comment` node.
    pub fn set_text_data(&mut self, handle: NodeHandle, data: &str) -> Result<(), DomError> {
        let boxed: Box<str> = data.into();
        let new_len = boxed.len() as u64;
        let node = self.resolve_mut(handle)?;
        let previous_len = match &mut node.payload {
            NodePayload::Text { data } | NodePayload::Comment { data } => {
                let previous = data.len() as u64;
                *data = boxed;
                previous
            }
            _ => return Err(DomError::WrongKind),
        };
        self.adjust_bytes(previous_len, new_len);
        self.bump_revision();
        self.touch(handle)?;
        self.notify_changed(handle, NodeChange::TextChanged);
        Ok(())
    }

    // ---- tree mutation --------------------------------------------------

    pub fn insert_before(
        &mut self,
        parent: NodeHandle,
        new_child: NodeHandle,
        reference: Option<NodeHandle>,
    ) -> Result<(), DomError> {
        self.validate_new_child(parent, new_child, &[new_child])?;
        if let Some(reference) = reference {
            let reference_data = self.resolve(reference)?;
            if reference_data.links.parent != Some(parent) {
                return Err(DomError::NotFound);
            }
        }
        // Inserting a node immediately before itself (only reachable when
        // it is already exactly at that position) is a genuine no-op;
        // proceeding would splice a node's sibling pointer to itself.
        if reference == Some(new_child) {
            return Ok(());
        }

        if self.resolve(new_child)?.links.parent.is_some() {
            self.detach_internal(new_child)?;
        }
        self.splice_before(parent, new_child, reference)?;

        self.bump_revision();
        self.touch(new_child)?;
        self.notify_changed(new_child, NodeChange::Inserted);
        Ok(())
    }

    pub fn append_child(
        &mut self,
        parent: NodeHandle,
        new_child: NodeHandle,
    ) -> Result<(), DomError> {
        self.insert_before(parent, new_child, None)
    }

    pub fn remove_child(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), DomError> {
        let child_data = self.resolve(child)?;
        if child_data.links.parent != Some(parent) {
            return Err(DomError::NotFound);
        }
        self.resolve(parent)?;

        self.detach_internal(child)?;
        self.bump_revision();
        self.touch(child)?;
        self.notify_changed(child, NodeChange::Detached);
        Ok(())
    }

    pub fn replace_child(
        &mut self,
        parent: NodeHandle,
        new_child: NodeHandle,
        old_child: NodeHandle,
    ) -> Result<(), DomError> {
        let old_parent = self.resolve(old_child)?.links.parent;
        if old_parent != Some(parent) {
            return Err(DomError::NotFound);
        }
        self.resolve(parent)?;

        if new_child == old_child {
            // No net change; still requires the arguments to be valid,
            // which the checks above already established.
            return Ok(());
        }

        self.validate_new_child(parent, new_child, &[new_child, old_child])?;

        let old_next = self.resolve(old_child)?.links.next_sibling;

        if old_next == Some(new_child) {
            // `new_child` is already immediately after `old_child`;
            // detaching `old_child` leaves `new_child` in the correct final
            // position with no further relinking. Handling this separately
            // avoids splicing `new_child` in "before itself" (see the
            // `insert_before` self-reference guard above for the same
            // class of bug).
            self.detach_internal(old_child)?;
        } else {
            let reference = match old_next {
                Some(candidate) => match self.resolve(candidate) {
                    Ok(data) if data.links.parent == Some(parent) => Some(candidate),
                    _ => None,
                },
                None => None,
            };
            self.detach_internal(old_child)?;
            if self.resolve(new_child)?.links.parent.is_some() {
                self.detach_internal(new_child)?;
            }
            self.splice_before(parent, new_child, reference)?;
        }

        self.bump_revision();
        self.touch(old_child)?;
        self.touch(new_child)?;
        self.notify_changed(old_child, NodeChange::Detached);
        self.notify_changed(new_child, NodeChange::Inserted);
        Ok(())
    }

    /// Moves the subtree rooted at `node` from `source` into `self`,
    /// freeing `node` and every descendant's slot in `source` (so every
    /// other handle into that subtree becomes stale) and re-creating an
    /// equivalent, fresh, detached subtree in `self` with re-interned
    /// atoms. Returns the new root handle; the caller still must
    /// `insert_before`/`append_child` it somewhere in `self`'s tree.
    ///
    /// Read-only collection from `source` happens entirely before any
    /// mutation of either document, and the new subtree is fully built in
    /// `self` before anything is freed in `source` — so a failure here
    /// (bad/stale `node`, or a subtree deeper than [`MAX_ANCESTOR_WALK`])
    /// never partially frees `source` or leaves an orphaned partial subtree
    /// in `self` (M2-T05 security review, finding 3b).
    ///
    /// Known, documented limitation (M2-T05 security review, finding 2b):
    /// because this is a free-and-recreate move rather than an in-place
    /// relocation, the returned handle is a *new* handle, not the same
    /// handle the node had in `source`. This cannot preserve JS object
    /// identity across `adoptNode` the way the DOM spec requires
    /// (`document.adoptNode(n) === n`). `self.notify_changed` fires
    /// [`NodeChange::Adopted`] on the new handle carrying the old handle,
    /// specifically so a future V8-bridge wrapper cache (M2-T06/T07) can
    /// re-key an existing wrapper instead of only seeing an unrelated
    /// create+free pair; full spec-identity preservation, if required,
    /// needs a binding-layer decision beyond this crate's scope.
    pub fn adopt_node(
        &mut self,
        source: &mut Document,
        node: NodeHandle,
    ) -> Result<NodeHandle, DomError> {
        if source.id == self.id {
            return Err(DomError::SameDocument);
        }

        let (owned, handles) = collect_subtree(source, node)?;
        let new_root = self.materialize(owned);
        self.bump_revision();
        self.notify_changed(new_root, NodeChange::Adopted { previous: node });

        if source.resolve(node)?.links.parent.is_some() {
            source.detach_internal(node)?;
        }
        for handle in handles {
            let freed = source.arena.free(handle)?;
            source.adjust_bytes(NODE_BASE_COST + payload_bytes(&freed.payload), 0);
            if let Some(observer) = &source.observer {
                observer.on_node_freed(handle);
            }
        }
        source.bump_revision();

        Ok(new_root)
    }

    /// Deep- or shallow-clones `node` (which must belong to `self`) into a
    /// fresh, detached subtree in the same document. The depth-bound check
    /// (`deep` case) is a read-only pre-pass over the existing tree before
    /// any new node is allocated (see `adopt_node`'s doc comment for the
    /// same reasoning): an over-limit clone leaves `memory_usage()`
    /// unchanged rather than leaking orphaned slots.
    pub fn clone_node(&mut self, node: NodeHandle, deep: bool) -> Result<NodeHandle, DomError> {
        let owned = if deep {
            collect_subtree(self, node)?.0
        } else {
            let data = self.resolve(node)?;
            OwnedNode {
                payload: to_owned_payload(self, &data.payload),
                children: Vec::new(),
            }
        };
        self.bump_revision();
        Ok(self.materialize(owned))
    }

    /// Reclaims a single node's slot. Valid only on a node that is
    /// currently detached (no parent) and has no children — the missing
    /// half of the ordinary (non-`adopt`, non-`close`) node lifecycle,
    /// added per the M2-T05 security review, finding 4: without this, a
    /// `remove_child`'d-and-abandoned node (the most common real-world DOM
    /// mutation pattern) had no path back to reclamation, and
    /// `WrapperObserver::on_node_freed` never fired for it. Intended caller:
    /// the future V8-bridge wrapper finalizer (M2-T06/T07), once a wrapper
    /// for a detached node becomes unreachable.
    pub fn destroy_node(&mut self, handle: NodeHandle) -> Result<(), DomError> {
        if handle == self.root {
            return Err(DomError::HierarchyViolation);
        }
        let data = self.resolve(handle)?;
        if data.links.parent.is_some() {
            return Err(DomError::NodeStillAttached);
        }
        if data.links.first_child.is_some() {
            return Err(DomError::NodeHasChildren);
        }
        let freed = self.arena.free(handle)?;
        self.adjust_bytes(NODE_BASE_COST + payload_bytes(&freed.payload), 0);
        self.bump_revision();
        if let Some(observer) = &self.observer {
            observer.on_node_freed(handle);
        }
        Ok(())
    }

    // ---- internal link manipulation ------------------------------------

    /// Validates that `new_child` may be inserted under `parent`: `parent`
    /// is a kind that can hold children, `new_child` is not `parent` and
    /// not an ancestor of `parent` (no cycle), and — if `parent` is the
    /// document node — the at-most-one-`Element`/at-most-one-`DocumentType`
    /// constraint holds once every handle in `ignore` is excluded from the
    /// count (so repositioning a node that is already the sole element, or
    /// replacing it with a different element, is not falsely rejected —
    /// M2-T05 security review, finding 3c). Read-only; never mutates.
    fn validate_new_child(
        &self,
        parent: NodeHandle,
        new_child: NodeHandle,
        ignore: &[NodeHandle],
    ) -> Result<(), DomError> {
        let parent_data = self.resolve(parent)?;
        let new_child_data = self.resolve(new_child)?;

        if !matches!(
            parent_data.kind(),
            NodeKind::Document | NodeKind::Element | NodeKind::DocumentFragment
        ) {
            return Err(DomError::HierarchyViolation);
        }
        if parent == new_child {
            return Err(DomError::HierarchyViolation);
        }
        self.ensure_not_ancestor(parent, new_child)?;

        if parent_data.kind() == NodeKind::Document {
            match new_child_data.kind() {
                NodeKind::Element => {
                    if self.children_of_kind_excluding(parent, NodeKind::Element, ignore)? {
                        return Err(DomError::HierarchyViolation);
                    }
                }
                NodeKind::DocumentType => {
                    if self.children_of_kind_excluding(parent, NodeKind::DocumentType, ignore)? {
                        return Err(DomError::HierarchyViolation);
                    }
                }
                NodeKind::Text => return Err(DomError::HierarchyViolation),
                NodeKind::DocumentFragment | NodeKind::Document => {
                    return Err(DomError::HierarchyViolation)
                }
                NodeKind::Comment => {}
            }
        }
        Ok(())
    }

    /// Walks from `parent` up through `.parent` links; fails with
    /// `HierarchyViolation` if `candidate` is encountered (that would be a
    /// cycle), or `DepthLimitExceeded` if the walk exceeds
    /// [`MAX_ANCESTOR_WALK`] without terminating.
    fn ensure_not_ancestor(
        &self,
        parent: NodeHandle,
        candidate: NodeHandle,
    ) -> Result<(), DomError> {
        let mut current = Some(parent);
        let mut steps = 0usize;
        while let Some(handle) = current {
            if handle == candidate {
                return Err(DomError::HierarchyViolation);
            }
            steps += 1;
            if steps > MAX_ANCESTOR_WALK {
                return Err(DomError::DepthLimitExceeded);
            }
            current = self.resolve(handle)?.links.parent;
        }
        Ok(())
    }

    fn children_of_kind_excluding(
        &self,
        parent: NodeHandle,
        kind: NodeKind,
        ignore: &[NodeHandle],
    ) -> Result<bool, DomError> {
        let mut current = self.resolve(parent)?.links.first_child;
        let mut steps = 0usize;
        while let Some(handle) = current {
            steps += 1;
            if steps > MAX_SIBLING_SCAN {
                return Err(DomError::DepthLimitExceeded);
            }
            let data = self.resolve(handle)?;
            if !ignore.contains(&handle) && data.kind() == kind {
                return Ok(true);
            }
            current = data.links.next_sibling;
        }
        Ok(false)
    }

    /// Unlinks `handle` from its current parent's child list (if any),
    /// clearing its own parent/sibling links. Does not free the slot — the
    /// node remains resolvable, just detached ("valid-but-detached").
    /// Always re-reads live links; never acts on caller-cached positions.
    fn detach_internal(&mut self, handle: NodeHandle) -> Result<(), DomError> {
        let (parent, previous, next) = {
            let data = self.resolve(handle)?;
            (
                data.links.parent,
                data.links.previous_sibling,
                data.links.next_sibling,
            )
        };
        if let Some(previous) = previous {
            self.resolve_mut(previous)?.links.next_sibling = next;
        } else if let Some(parent) = parent {
            self.resolve_mut(parent)?.links.first_child = next;
        }
        if let Some(next) = next {
            self.resolve_mut(next)?.links.previous_sibling = previous;
        } else if let Some(parent) = parent {
            self.resolve_mut(parent)?.links.last_child = previous;
        }
        let data = self.resolve_mut(handle)?;
        data.links.parent = None;
        data.links.previous_sibling = None;
        data.links.next_sibling = None;
        Ok(())
    }

    /// Splices `child` into `parent`'s child list immediately before
    /// `reference`, or appends it if `reference` is `None`. Assumes `child`
    /// is currently detached and `reference` (if given) currently belongs
    /// to `parent` — both already established by the caller's validate
    /// phase, re-checked live at each step here.
    fn splice_before(
        &mut self,
        parent: NodeHandle,
        child: NodeHandle,
        reference: Option<NodeHandle>,
    ) -> Result<(), DomError> {
        match reference {
            None => self.link_append(parent, child),
            Some(reference) => {
                let previous = self.resolve(reference)?.links.previous_sibling;
                {
                    let child_data = self.resolve_mut(child)?;
                    child_data.links.parent = Some(parent);
                    child_data.links.previous_sibling = previous;
                    child_data.links.next_sibling = Some(reference);
                }
                match previous {
                    Some(previous) => self.resolve_mut(previous)?.links.next_sibling = Some(child),
                    None => self.resolve_mut(parent)?.links.first_child = Some(child),
                }
                self.resolve_mut(reference)?.links.previous_sibling = Some(child);
                Ok(())
            }
        }
    }

    /// Appends `child` (assumed currently detached) as `parent`'s new last
    /// child.
    fn link_append(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), DomError> {
        let previous_last = self.resolve(parent)?.links.last_child;
        {
            let parent_data = self.resolve_mut(parent)?;
            if parent_data.links.first_child.is_none() {
                parent_data.links.first_child = Some(child);
            }
            parent_data.links.last_child = Some(child);
        }
        if let Some(previous_last) = previous_last {
            self.resolve_mut(previous_last)?.links.next_sibling = Some(child);
        }
        let child_data = self.resolve_mut(child)?;
        child_data.links.parent = Some(parent);
        child_data.links.previous_sibling = previous_last;
        child_data.links.next_sibling = None;
        Ok(())
    }

    /// Builds a fresh, detached subtree in `self` from an already-collected
    /// [`OwnedNode`] tree. Infallible: `owned`'s depth was already bounded
    /// by [`MAX_ANCESTOR_WALK`] during collection (`collect_subtree`), and
    /// inserting into `self`'s own arena from already-owned data cannot
    /// fail. Iterative (explicit stack), not recursive.
    fn materialize(&mut self, root: OwnedNode) -> NodeHandle {
        struct Frame {
            handle: NodeHandle,
            children: std::vec::IntoIter<OwnedNode>,
        }

        let root_payload = self.build_owned_payload(root.payload);
        let root_handle = self.insert_node(root_payload);
        let mut stack = vec![Frame {
            handle: root_handle,
            children: root.children.into_iter(),
        }];

        while let Some(frame) = stack.last_mut() {
            match frame.children.next() {
                Some(child_owned) => {
                    let parent_handle = frame.handle;
                    let child_payload = self.build_owned_payload(child_owned.payload);
                    let child_handle = self.insert_node(child_payload);
                    // `parent_handle`/`child_handle` were both just created
                    // in this arena, so this can only fail if the arena
                    // itself is corrupt; treat that as unreachable rather
                    // than threading a Result through an otherwise
                    // infallible builder.
                    let _ = self.link_append(parent_handle, child_handle);
                    stack.push(Frame {
                        handle: child_handle,
                        children: child_owned.children.into_iter(),
                    });
                }
                None => {
                    stack.pop();
                }
            }
        }
        root_handle
    }

    fn build_owned_payload(&mut self, payload: OwnedPayload) -> NodePayload {
        match payload {
            OwnedPayload::Document => NodePayload::Document,
            OwnedPayload::DocumentFragment => NodePayload::DocumentFragment,
            OwnedPayload::DocumentType {
                name,
                public_id,
                system_id,
            } => NodePayload::DocumentType {
                name,
                public_id,
                system_id,
            },
            OwnedPayload::Text(data) => NodePayload::Text { data },
            OwnedPayload::Comment(data) => NodePayload::Comment { data },
            OwnedPayload::Element {
                tag,
                namespace,
                attributes,
            } => {
                let tag_atom = self.interner.intern(&tag);
                let mut map = AttributeMap::new();
                for (name, value) in attributes {
                    let atom = self.interner.intern(&name);
                    map.set(atom, value);
                }
                NodePayload::Element {
                    tag: tag_atom,
                    namespace,
                    attributes: map,
                }
            }
        }
    }
}

/// Owned, document-independent snapshot of one node's payload, used to
/// carry a subtree between arenas (`clone_node`, `adopt_node`) without
/// holding any live borrow of either document across the copy.
struct OwnedNode {
    payload: OwnedPayload,
    children: Vec<OwnedNode>,
}

enum OwnedPayload {
    Document,
    DocumentFragment,
    DocumentType {
        name: Box<str>,
        public_id: Box<str>,
        system_id: Box<str>,
    },
    Text(Box<str>),
    Comment(Box<str>),
    Element {
        tag: String,
        namespace: Namespace,
        attributes: Vec<(String, Box<str>)>,
    },
}

fn to_owned_payload(doc: &Document, payload: &NodePayload) -> OwnedPayload {
    match payload {
        NodePayload::Document => OwnedPayload::Document,
        NodePayload::DocumentFragment => OwnedPayload::DocumentFragment,
        NodePayload::DocumentType {
            name,
            public_id,
            system_id,
        } => OwnedPayload::DocumentType {
            name: name.clone(),
            public_id: public_id.clone(),
            system_id: system_id.clone(),
        },
        NodePayload::Text { data } => OwnedPayload::Text(data.clone()),
        NodePayload::Comment { data } => OwnedPayload::Comment(data.clone()),
        NodePayload::Element {
            tag,
            namespace,
            attributes,
        } => {
            let tag_name = resolve_atom(doc, *tag).to_string();
            let attrs = attribute_pairs(doc, attributes);
            OwnedPayload::Element {
                tag: tag_name,
                namespace: *namespace,
                attributes: attrs,
            }
        }
    }
}

fn resolve_atom(doc: &Document, atom: Atom) -> &str {
    doc.interner.resolve(atom)
}

fn attribute_pairs(doc: &Document, attributes: &AttributeMap) -> Vec<(String, Box<str>)> {
    attributes.pairs_for_clone(doc)
}

/// In-order handles of `parent`'s current children.
pub(crate) fn collect_children(
    doc: &Document,
    parent: NodeHandle,
) -> Result<Vec<NodeHandle>, DomError> {
    let mut children = Vec::new();
    let mut current = doc.resolve(parent)?.links.first_child;
    let mut steps = 0usize;
    while let Some(handle) = current {
        steps += 1;
        if steps > MAX_SIBLING_SCAN {
            return Err(DomError::DepthLimitExceeded);
        }
        let data = doc.resolve(handle)?;
        children.push(handle);
        current = data.links.next_sibling;
    }
    Ok(children)
}

/// Read-only, iterative (explicit stack, not recursion) collection of the
/// subtree rooted at `root` into an owned, document-independent tree, plus
/// a flat list of every handle visited (used by `adopt_node` to free the
/// originals). Entirely a read pass over `doc`: never mutates it, and never
/// touches a destination document — the depth bound therefore always fires
/// before any destination allocation happens (M2-T05 security review,
/// finding 3b).
fn collect_subtree(
    doc: &Document,
    root: NodeHandle,
) -> Result<(OwnedNode, Vec<NodeHandle>), DomError> {
    struct Pending {
        payload: OwnedPayload,
        children: Vec<OwnedNode>,
        remaining: Vec<NodeHandle>,
    }

    let mut visited = Vec::new();
    let root_data = doc.resolve(root)?;
    visited.push(root);
    let mut stack = vec![Pending {
        payload: to_owned_payload(doc, &root_data.payload),
        children: Vec::new(),
        remaining: collect_children(doc, root)?.into_iter().rev().collect(),
    }];

    loop {
        if stack.len() > MAX_ANCESTOR_WALK {
            return Err(DomError::DepthLimitExceeded);
        }
        let next_child = match stack.last_mut() {
            Some(top) => top.remaining.pop(),
            None => None,
        };
        match next_child {
            Some(child_handle) => {
                let child_data = doc.resolve(child_handle)?;
                visited.push(child_handle);
                stack.push(Pending {
                    payload: to_owned_payload(doc, &child_data.payload),
                    children: Vec::new(),
                    remaining: collect_children(doc, child_handle)?
                        .into_iter()
                        .rev()
                        .collect(),
                });
            }
            None => {
                let Some(finished) = stack.pop() else {
                    return Err(DomError::HierarchyViolation);
                };
                let node = OwnedNode {
                    payload: finished.payload,
                    children: finished.children,
                };
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => return Ok((node, visited)),
                }
            }
        }
    }
}
