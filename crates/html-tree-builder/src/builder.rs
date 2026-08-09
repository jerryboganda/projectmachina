//! `TreeBuilder`: the dispatch loop, node-creation/insertion primitives
//! shared by every insertion-mode handler, and the pause/resume driving
//! methods (`feed`/`finish`/`resume_after_script`, design §6).
//!
//! `TreeBuilder` never owns a `Document` or `Tokenizer` — every driving
//! method takes both as `&mut` parameters per call. Handles
//! (`ElementHandle`, `NodeHandle`) are `Copy`, so no live borrow persists
//! between calls; pausing at a script checkpoint is just "return from
//! `feed`/`finish`", not a self-referential struct.

use machina_dom::{Document, ElementHandle, Namespace, NodeHandle, NodeKind};
use machina_html::{Attribute, Token, TokenizerEvent};

use crate::active_formatting::{ActiveFormattingElements, FormattingEntry};
use crate::checkpoint::{ScriptCheckpoint, TreeBuilderOutcome};
use crate::diagnostics::Diagnostic;
use crate::error::TreeBuilderError;
use crate::foster_parent::{foster_parent_target, FosterTarget};
use crate::limits::TreeBuilderLimits;
use crate::modes::{Dispatch, InsertionMode};
use crate::open_elements::{OpenElementEntry, OpenElementsStack};

pub struct TreeBuilder {
    pub(crate) mode: InsertionMode,
    pub(crate) original_mode: Option<InsertionMode>,
    pub(crate) template_modes: Vec<InsertionMode>,
    pub(crate) open_elements: OpenElementsStack,
    pub(crate) afe: ActiveFormattingElements,
    pub(crate) head_element: Option<ElementHandle>,
    pub(crate) form_element: Option<ElementHandle>,
    pub(crate) scripting_enabled: bool,
    pub(crate) frameset_ok: bool,
    pub(crate) paused: Option<ScriptCheckpoint>,
    pub(crate) poisoned: bool,
    pub(crate) limits: TreeBuilderLimits,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) pending_table_text: String,
    pub(crate) pending_table_text_had_non_whitespace: bool,
    pub(crate) pending_table_text_original_mode: InsertionMode,
    pub(crate) is_fragment: bool,
    pub(crate) done: bool,
}

pub(crate) fn attrs_to_pairs(attrs: &[Attribute]) -> Vec<(String, String)> {
    attrs
        .iter()
        .map(|a| (a.name.clone(), a.value.clone()))
        .collect()
}

impl TreeBuilder {
    pub fn new(scripting_enabled: bool) -> Self {
        Self::with_limits(scripting_enabled, TreeBuilderLimits::default())
    }

    pub fn with_limits(scripting_enabled: bool, limits: TreeBuilderLimits) -> Self {
        Self {
            mode: InsertionMode::Initial,
            original_mode: None,
            template_modes: Vec::new(),
            open_elements: OpenElementsStack::new(),
            afe: ActiveFormattingElements::new(),
            head_element: None,
            form_element: None,
            scripting_enabled,
            frameset_ok: true,
            paused: None,
            poisoned: false,
            limits,
            diagnostics: Vec::new(),
            pending_table_text: String::new(),
            pending_table_text_had_non_whitespace: false,
            pending_table_text_original_mode: InsertionMode::InBody,
            is_fragment: false,
            done: false,
        }
    }

    /// Fragment parsing (design §4's `new_fragment`), simplified: creates a
    /// detached `<html>` root in `doc` to anchor the open-elements stack
    /// (never linked to `doc`'s document node — the caller is responsible
    /// for relocating the fragment's resulting children, e.g. via repeated
    /// `Document::append_child`/`adopt_node`), and seeds the insertion mode
    /// from `context_tag` using a representative subset of WHATWG's "reset
    /// the insertion mode appropriately" table (select/table/body cases;
    /// anything else starts in `InBody`). This is a one-time seed, not the
    /// full dynamically-re-run "reset the insertion mode appropriately"
    /// algorithm the spec invokes elsewhere (documented simplification).
    pub fn new_fragment(
        doc: &mut Document,
        context_tag: &str,
        context_namespace: Namespace,
        scripting_enabled: bool,
    ) -> Result<(Self, ElementHandle), TreeBuilderError> {
        let mut builder = Self::new(scripting_enabled);
        builder.is_fragment = true;
        let root = doc
            .create_element_ns(Namespace::Html, "html")
            .map_err(|e| builder.poison(format!("fragment root creation failed: {e}")))?;
        builder.open_elements.push(OpenElementEntry {
            handle: root,
            tag: "html".to_string(),
            namespace: Namespace::Html,
        });
        builder.mode = match (context_namespace, context_tag) {
            (Namespace::Html, "select") => InsertionMode::InSelect,
            (Namespace::Html, "table") => InsertionMode::InTable,
            (Namespace::Html, "tbody" | "thead" | "tfoot") => InsertionMode::InTableBody,
            (Namespace::Html, "tr") => InsertionMode::InRow,
            (Namespace::Html, "td" | "th") => InsertionMode::InCell,
            (Namespace::Html, "head") => InsertionMode::InHead,
            (Namespace::Html, "html") => InsertionMode::BeforeHead,
            _ => InsertionMode::InBody,
        };
        Ok((builder, root))
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// The `<html>` document-element handle, once inserted (after
    /// `BeforeHtml` runs).
    pub fn document_element(&self) -> Option<ElementHandle> {
        self.open_elements.html_element()
    }

    pub fn is_paused(&self) -> bool {
        self.paused.is_some()
    }

    // ---- driving methods (design §6) -----------------------------------

    pub fn feed(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut machina_html::Tokenizer,
        chunk: &[u8],
    ) -> Result<TreeBuilderOutcome, TreeBuilderError> {
        self.check_not_poisoned()?;
        if self.paused.is_some() {
            return Err(TreeBuilderError::AlreadyPaused);
        }
        tokenizer.feed(chunk);
        self.drive(doc, tokenizer)
    }

    pub fn finish(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut machina_html::Tokenizer,
    ) -> Result<TreeBuilderOutcome, TreeBuilderError> {
        self.check_not_poisoned()?;
        if self.paused.is_some() {
            return Err(TreeBuilderError::AlreadyPaused);
        }
        tokenizer.finish();
        self.drive(doc, tokenizer)
    }

    pub fn resume_after_script(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut machina_html::Tokenizer,
    ) -> Result<TreeBuilderOutcome, TreeBuilderError> {
        self.check_not_poisoned()?;
        if self.paused.take().is_none() {
            return Err(TreeBuilderError::NotPaused);
        }
        self.drive(doc, tokenizer)
    }

    fn check_not_poisoned(&self) -> Result<(), TreeBuilderError> {
        if self.poisoned {
            Err(TreeBuilderError::Poisoned)
        } else {
            Ok(())
        }
    }

    pub(crate) fn poison(&mut self, detail: String) -> TreeBuilderError {
        self.poisoned = true;
        TreeBuilderError::Internal(detail)
    }

    /// Pulls tokens from `tokenizer` and dispatches each through the
    /// insertion-mode handlers until the tokenizer needs more input, an
    /// HTML-namespace `</script>` end tag closes (a `ScriptCheckpoint`), or
    /// `Token::Eof` is processed (`Done`).
    fn drive(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut machina_html::Tokenizer,
    ) -> Result<TreeBuilderOutcome, TreeBuilderError> {
        if self.done {
            return Ok(TreeBuilderOutcome::Done);
        }
        loop {
            let Some(event) = tokenizer.next_event() else {
                return Ok(TreeBuilderOutcome::NeedsMoreInput);
            };
            let token = match event {
                TokenizerEvent::Diagnostic(d) => {
                    self.diagnostics.push(Diagnostic::Tokenizer(d));
                    continue;
                }
                TokenizerEvent::Token(token) => token,
            };
            let is_eof = matches!(token, Token::Eof);
            self.dispatch(doc, tokenizer, token)?;
            if let Some(checkpoint) = self.paused {
                return Ok(TreeBuilderOutcome::ScriptCheckpoint(checkpoint));
            }
            if is_eof {
                self.done = true;
                return Ok(TreeBuilderOutcome::Done);
            }
        }
    }

    /// Dispatches one token through the current insertion mode, following
    /// `Dispatch::Reprocess` hops up to `max_reprocess_hops` times (design
    /// §1, §7f) — bounded, non-recursive.
    fn dispatch(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut machina_html::Tokenizer,
        token: Token,
    ) -> Result<(), TreeBuilderError> {
        let mut mode = self.mode;
        let mut hops = 0usize;
        loop {
            let dispatch = self.run_mode(doc, tokenizer, mode, &token)?;
            match dispatch {
                Dispatch::Consumed => {
                    return Ok(());
                }
                Dispatch::Reprocess(next_mode) => {
                    hops += 1;
                    if hops > self.limits.max_reprocess_hops {
                        return Err(self.poison(format!(
                            "reprocess hop bound ({}) exceeded dispatching a token",
                            self.limits.max_reprocess_hops
                        )));
                    }
                    self.mode = next_mode;
                    mode = next_mode;
                }
            }
        }
    }

    // ---- shared insertion primitives ------------------------------------

    pub(crate) fn current_node_handle(&self) -> Option<NodeHandle> {
        self.open_elements.current_handle().map(|h| h.node_handle())
    }

    fn current_insertion_parent(&self, doc: &Document) -> NodeHandle {
        self.current_node_handle().unwrap_or_else(|| doc.root())
    }

    pub(crate) fn insertion_target(&self, doc: &Document, foster: bool) -> FosterTarget {
        if foster {
            foster_parent_target(&self.open_elements, doc)
        } else {
            FosterTarget {
                parent: self.current_insertion_parent(doc),
                before: None,
            }
        }
    }

    pub(crate) fn insert_node_at_target(
        &mut self,
        doc: &mut Document,
        target: FosterTarget,
        node: NodeHandle,
    ) -> Result<(), TreeBuilderError> {
        let result = match target.before {
            Some(before) => doc.insert_before(target.parent, node, Some(before)),
            None => doc.append_child(target.parent, node),
        };
        result.map_err(|e| self.poison(format!("insert_node_at_target failed: {e}")))
    }

    pub(crate) fn append_child_checked(
        &mut self,
        doc: &mut Document,
        parent: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), TreeBuilderError> {
        doc.append_child(parent, child)
            .map_err(|e| self.poison(format!("append_child failed: {e}")))
    }

    /// Creates an element and its attributes without inserting it into the
    /// DOM tree or pushing it onto the open-elements stack. Used by the
    /// adoption agency algorithm, which does its own placement/bookkeeping.
    pub(crate) fn create_detached_element(
        &mut self,
        doc: &mut Document,
        tag: &str,
        namespace: Namespace,
        attrs: &[(String, String)],
    ) -> Result<ElementHandle, TreeBuilderError> {
        let handle = doc
            .create_element_ns(namespace, tag)
            .map_err(|e| self.poison(format!("create_element_ns <{tag}> failed: {e}")))?;
        for (name, value) in attrs {
            doc.set_attribute(handle, name, value)
                .map_err(|e| self.poison(format!("set_attribute {name} on <{tag}> failed: {e}")))?;
        }
        Ok(handle)
    }

    /// Moves every current child of `from` to become a child of `to`, in
    /// order. Used by the adoption agency algorithm's "take all of
    /// furthestBlock's child nodes" step.
    pub(crate) fn move_all_children(
        &mut self,
        doc: &mut Document,
        from: NodeHandle,
        to: NodeHandle,
    ) -> Result<(), TreeBuilderError> {
        let children = doc
            .children(from)
            .map_err(|e| self.poison(format!("move_all_children: children() failed: {e}")))?;
        for child in children {
            self.append_child_checked(doc, to, child)?;
        }
        Ok(())
    }

    /// The adoption agency algorithm's "insert whichever last node is now"
    /// step: places `node` under `common_ancestor` using foster parenting
    /// if `common_ancestor` is a table-context element (design §3's shared
    /// foster-parent code path, reused here per the spec's own note that
    /// foster parenting applies inside the adoption agency algorithm too).
    pub(crate) fn place_under_common_ancestor(
        &mut self,
        doc: &mut Document,
        common_ancestor: ElementHandle,
        node: ElementHandle,
    ) -> Result<(), TreeBuilderError> {
        let tag = doc
            .tag_name(common_ancestor)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let foster = matches!(tag.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr");
        if foster {
            let target = foster_parent_target(&self.open_elements, doc);
            self.insert_node_at_target(doc, target, node.node_handle())
        } else {
            self.append_child_checked(doc, common_ancestor.node_handle(), node.node_handle())
        }
    }

    /// Creates and inserts an element for the given tag/namespace/attrs.
    /// If `push_open` and the open-elements stack is already at
    /// `max_open_elements_depth`, records
    /// `Diagnostic::NestingLimitExceeded` and returns `Ok(None)` *without*
    /// touching the DOM at all (design §7a/§7b — the ceiling is checked
    /// strictly before any DOM call).
    pub(crate) fn create_and_insert_element(
        &mut self,
        doc: &mut Document,
        tag: &str,
        namespace: Namespace,
        attrs: &[(String, String)],
        foster: bool,
        push_open: bool,
    ) -> Result<Option<ElementHandle>, TreeBuilderError> {
        if push_open && self.open_elements.len() >= self.limits.max_open_elements_depth {
            self.diagnostics.push(Diagnostic::NestingLimitExceeded {
                local_name: tag.to_string(),
                depth: self.open_elements.len(),
            });
            return Ok(None);
        }
        let handle = doc
            .create_element_ns(namespace, tag)
            .map_err(|e| self.poison(format!("create_element_ns <{tag}> failed: {e}")))?;
        for (name, value) in attrs {
            doc.set_attribute(handle, name, value)
                .map_err(|e| self.poison(format!("set_attribute {name} on <{tag}> failed: {e}")))?;
        }
        let target = self.insertion_target(doc, foster);
        self.insert_node_at_target(doc, target, handle.node_handle())?;
        if push_open {
            self.open_elements.push(OpenElementEntry {
                handle,
                tag: tag.to_string(),
                namespace,
            });
        }
        Ok(Some(handle))
    }

    pub(crate) fn insert_html_element(
        &mut self,
        doc: &mut Document,
        tag: &str,
        attrs: &[Attribute],
    ) -> Result<Option<ElementHandle>, TreeBuilderError> {
        let pairs = attrs_to_pairs(attrs);
        self.create_and_insert_element(doc, tag, Namespace::Html, &pairs, false, true)
    }

    pub(crate) fn insert_html_element_foster(
        &mut self,
        doc: &mut Document,
        tag: &str,
        attrs: &[Attribute],
    ) -> Result<Option<ElementHandle>, TreeBuilderError> {
        let pairs = attrs_to_pairs(attrs);
        self.create_and_insert_element(doc, tag, Namespace::Html, &pairs, true, true)
    }

    pub(crate) fn insert_void_html_element(
        &mut self,
        doc: &mut Document,
        tag: &str,
        attrs: &[Attribute],
        foster: bool,
    ) -> Result<(), TreeBuilderError> {
        let pairs = attrs_to_pairs(attrs);
        if self
            .create_and_insert_element(doc, tag, Namespace::Html, &pairs, foster, true)?
            .is_some()
        {
            self.open_elements.pop();
        }
        Ok(())
    }

    pub(crate) fn insert_comment(
        &mut self,
        doc: &mut Document,
        data: &str,
        foster: bool,
    ) -> Result<(), TreeBuilderError> {
        let handle = doc.create_comment(data);
        let target = self.insertion_target(doc, foster);
        self.insert_node_at_target(doc, target, handle)
    }

    pub(crate) fn insert_comment_at(
        &mut self,
        doc: &mut Document,
        data: &str,
        parent: NodeHandle,
    ) -> Result<(), TreeBuilderError> {
        let handle = doc.create_comment(data);
        doc.append_child(parent, handle)
            .map_err(|e| self.poison(format!("insert_comment_at failed: {e}")))
    }

    pub(crate) fn insert_text(
        &mut self,
        doc: &mut Document,
        data: &str,
        foster: bool,
    ) -> Result<(), TreeBuilderError> {
        if data.is_empty() {
            return Ok(());
        }
        let target = self.insertion_target(doc, foster);
        let existing_last = match target.before {
            Some(before) => doc.node(before).ok().and_then(|n| n.previous_sibling()),
            None => doc.node(target.parent).ok().and_then(|n| n.last_child()),
        };
        if let Some(last) = existing_last {
            if let Ok(node_ref) = doc.node(last) {
                if node_ref.kind() == NodeKind::Text {
                    let mut combined = doc
                        .text_data(last)
                        .map_err(|e| self.poison(format!("text_data during merge failed: {e}")))?
                        .to_string();
                    combined.push_str(data);
                    doc.set_text_data(last, &combined).map_err(|e| {
                        self.poison(format!("set_text_data during merge failed: {e}"))
                    })?;
                    return Ok(());
                }
            }
        }
        let text_handle = doc.create_text(data);
        self.insert_node_at_target(doc, target, text_handle.node_handle())
    }

    /// Creates a new element for `tag`/`namespace`/`attrs` and pushes it
    /// onto the open-elements stack without going through the ordinary
    /// `AFE`-push path — used by "reconstruct the active formatting
    /// elements" and the adoption agency algorithm, which manage the AFE
    /// list themselves.
    pub(crate) fn create_element_and_push(
        &mut self,
        doc: &mut Document,
        tag: &str,
        namespace: Namespace,
        attrs: &[(String, String)],
    ) -> Result<Option<ElementHandle>, TreeBuilderError> {
        self.create_and_insert_element(doc, tag, namespace, attrs, false, true)
    }

    // ---- reconstruct active formatting elements (§13.2.4.3) -------------

    pub(crate) fn reconstruct_active_formatting_elements(
        &mut self,
        doc: &mut Document,
    ) -> Result<(), TreeBuilderError> {
        if self.afe.entries().is_empty() {
            return Ok(());
        }
        let last_index = self.afe.entries().len() - 1;
        match self.afe.entry_at(last_index) {
            Some(FormattingEntry::Marker) | None => return Ok(()),
            Some(FormattingEntry::Element { handle, .. }) => {
                if self.open_elements.contains_handle(*handle) {
                    return Ok(());
                }
            }
        }
        let mut index = last_index;
        while index > 0 {
            index -= 1;
            let should_advance = match self.afe.entry_at(index) {
                Some(FormattingEntry::Marker) => true,
                Some(FormattingEntry::Element { handle, .. }) => {
                    self.open_elements.contains_handle(*handle)
                }
                None => true,
            };
            if should_advance {
                index += 1;
                break;
            }
        }
        loop {
            let entry = self.afe.entry_at(index).cloned();
            if let Some(FormattingEntry::Element {
                tag,
                namespace,
                attrs,
                ..
            }) = entry
            {
                if let Some(new_handle) =
                    self.create_element_and_push(doc, &tag, namespace, &attrs)?
                {
                    self.afe.replace_at(
                        index,
                        FormattingEntry::Element {
                            handle: new_handle,
                            tag,
                            namespace,
                            attrs,
                        },
                    );
                } else {
                    // Nesting limit hit while reconstructing; stop rather
                    // than risk an inconsistent AFE/open-elements pairing.
                    break;
                }
            }
            if index == last_index {
                break;
            }
            index += 1;
        }
        Ok(())
    }

    // ---- misc shared helpers used by rules.rs ---------------------------

    pub(crate) fn close_p_element_if_in_button_scope(
        &mut self,
        doc: &mut Document,
    ) -> Result<(), TreeBuilderError> {
        if self.open_elements.has_in_button_scope("p") {
            self.close_p_element(doc)?;
        }
        Ok(())
    }

    pub(crate) fn close_p_element(&mut self, doc: &mut Document) -> Result<(), TreeBuilderError> {
        self.open_elements.pop_implied_end_tags(Some("p"), false);
        if self.open_elements.current().map(|e| e.tag.as_str()) != Some("p") {
            self.diagnostics.push(Diagnostic::ParseError {
                detail: "implied </p> did not land on a <p> element".to_string(),
            });
        }
        self.open_elements.pop_until_html_tag("p");
        let _ = doc;
        Ok(())
    }

    pub(crate) fn record_ignored(&mut self, local_name: &str, mode: &'static str) {
        self.diagnostics.push(Diagnostic::TokenIgnored {
            local_name: local_name.to_string(),
            mode,
        });
    }

    pub(crate) fn record_parse_error(&mut self, detail: impl Into<String>) {
        self.diagnostics.push(Diagnostic::ParseError {
            detail: detail.into(),
        });
    }

    /// Fires the tokenizer-text-content-state hook (design §5) for a
    /// start tag whose content model requires it, and switches to `Text`
    /// mode (`original_mode` remembers what to restore on exit).
    pub(crate) fn switch_to_text_mode(
        &mut self,
        tokenizer: &mut machina_html::Tokenizer,
        state: machina_html::TextContentState,
    ) {
        tokenizer.switch_to(state);
        self.original_mode = Some(self.mode);
        self.mode = InsertionMode::Text;
    }

    pub(crate) fn generic_raw_text_or_rcdata(
        &mut self,
        doc: &mut Document,
        tokenizer: &mut machina_html::Tokenizer,
        tag: &machina_html::TagToken,
        state: machina_html::TextContentState,
    ) -> Result<(), TreeBuilderError> {
        self.insert_html_element(doc, &tag.name, &tag.attributes)?;
        self.switch_to_text_mode(tokenizer, state);
        Ok(())
    }

    /// The WHATWG "any other end tag" algorithm (§13.2.6.4.7's `InBody`
    /// end-tag catch-all, also reused by the adoption agency algorithm
    /// when no matching formatting element is found). Pure open-elements-
    /// stack bookkeeping; touches no DOM node.
    pub(crate) fn any_other_end_tag_in_body(&mut self, target: &str) {
        for i in (0..self.open_elements.len()).rev() {
            let Some(entry) = self.open_elements.entry_at(i).cloned() else {
                break;
            };
            if entry.namespace == Namespace::Html && entry.tag == target {
                self.open_elements.pop_implied_end_tags(Some(target), false);
                if self.open_elements.current().map(|e| e.tag.as_str()) != Some(target) {
                    self.record_parse_error(format!("</{target}> did not match current node"));
                }
                self.open_elements.pop_until_html_tag(target);
                return;
            }
            if crate::special::is_special(&entry.tag, entry.namespace) {
                self.record_parse_error(format!(
                    "</{target}> ignored: hit special element <{}> before a match",
                    entry.tag
                ));
                return;
            }
        }
    }
}
