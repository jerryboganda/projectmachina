//! Semantic index extraction: roles, accessible names, headings, links,
//! forms, and interactive elements, derived by reading directly from the
//! live [`Document`] once per call — no parallel/cloned tree representation
//! is ever built (see the crate-level docs' "no duplicate DOM copies"
//! section).
//!
//! **Two full-document passes, both `O(document size)`:**
//! 1. [`build_label_map`] — collects every `<label for="...">`'s `for`
//!    value -> label element, so accessible-name resolution for form
//!    controls (pass 2) never needs its own per-control document search
//!    (which would risk building a CSS-selector string out of an untrusted
//!    attribute value — see that function's docs for why this crate
//!    deliberately avoids that).
//! 2. The main pass — walks every node once, and for each `Element` tests
//!    membership in a small compiled candidate selector (compiled once,
//!    reused for every element — the `machina_selectors` integration this
//!    task named explicitly) before doing any role/name computation, so the
//!    (relatively) expensive per-element logic only runs for elements that
//!    could plausibly carry a role.
//!
//! No incrementally-maintained index across calls (mirrors
//! `crates/selectors`'s own design §3 "no-indexing MVP" decision, and the
//! same `Revision`-stamped-result contract as `crates/selectors`'s
//! `QueryResult`/`XPathResult` — see this crate's evidence doc for the
//! query→mutate→requery test proving stale results are detectable via
//! `revision != document.revision()`).

use std::collections::HashMap;

use machina_dom::{Document, ElementHandle, NodeHandle, Revision};
use machina_selectors::css::{matches_compiled, parse_selector, SelectorList};

use crate::error::SemanticError;
use crate::limits::{MAX_ANCESTOR_LOOKUP_WALK, MAX_SEMANTIC_ITEMS};
use crate::role::{derive_accessible_name, derive_role, heading_level, INTERACTIVE_ROLES};
use crate::walk::walk_document_order;

/// One element with a derived (explicit `role="..."` or implicit
/// HTML-AAM-subset) role and accessible name, self-carrying the
/// [`ElementHandle`] it was derived from so a caller can act on it directly
/// (per the M2-T13 task briefing's "stable handles" requirement).
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub handle: ElementHandle,
    pub role: String,
    pub accessible_name: Option<String>,
    /// `Some(1..=6)` only when `role == "heading"`.
    pub heading_level: Option<u8>,
}

/// A link (`a[href]`) — the general [`SemanticNode`] fields plus the raw,
/// unresolved `href` attribute value (no base-URL resolution/canonicalization
/// is performed by this crate).
#[derive(Clone, Debug, PartialEq)]
pub struct LinkNode {
    pub handle: ElementHandle,
    pub accessible_name: Option<String>,
    pub href: String,
}

/// A `<form>` element and the form controls found inside it.
///
/// **Scope note**: `controls` is *descendant*-based only (every
/// input/select/textarea/button that is a DOM descendant of this form
/// element). HTML's `form="other-form-id"` attribute, which associates a
/// control with a form it is *not* a descendant of, is not resolved — a
/// disclosed, real gap (see `.agent-state/evidence/M2-T13.md`).
#[derive(Clone, Debug, PartialEq)]
pub struct FormNode {
    pub handle: ElementHandle,
    pub accessible_name: Option<String>,
    pub action: Option<String>,
    pub method: Option<String>,
    pub controls: Vec<ElementHandle>,
}

/// One extraction call's full semantic snapshot, self-stamped with the
/// [`Revision`] it was computed against — mirrors
/// `machina_selectors::query::QueryResult`'s contract exactly: staleness
/// detection is plain `index.revision != document.revision()`, never
/// guessed or implicit.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticIndex {
    pub revision: Revision,
    /// Every element this pass derived a role for, in document order.
    pub roles: Vec<SemanticNode>,
    /// Subset of `roles` with `role == "heading"`, in document order.
    pub headings: Vec<SemanticNode>,
    /// Every `a[href]`, in document order.
    pub links: Vec<LinkNode>,
    /// Every `<form>`, in document order.
    pub forms: Vec<FormNode>,
    /// Subset of `roles` whose role is in
    /// [`crate::role::INTERACTIVE_ROLES`], in document order.
    pub interactive_elements: Vec<SemanticNode>,
    /// `true` if any of the vectors above stopped accumulating items after
    /// hitting [`MAX_SEMANTIC_ITEMS`] (bounded output, not an error — the
    /// walk itself still completed).
    pub truncated: bool,
}

/// Fixed, hand-written (never caller-supplied) selector text naming every
/// tag/attribute this crate's role table (`role.rs`) knows how to derive a
/// role for. Compiled once per [`extract_semantic_index`] call via
/// `machina_selectors::css::parse_selector` and tested per element via
/// `matches_compiled` — the "use machina-selectors to locate candidates
/// rather than writing another tree-walk-and-match from scratch" approach
/// named in the M2-T13 task briefing.
const CANDIDATE_SELECTOR_TEXT: &str = "a, button, input, select, textarea, \
h1, h2, h3, h4, h5, h6, nav, main, form, img, ul, ol, li, table, summary, [role]";

fn candidate_selector() -> Result<SelectorList, SemanticError> {
    Ok(parse_selector(CANDIDATE_SELECTOR_TEXT)?)
}

/// Builds the `for`-attribute -> `<label>` element map used to resolve a
/// form control's accessible name via an explicit `<label for="...">`.
///
/// **Why this is a document-wide index pass rather than a per-control CSS
/// query**: a naive per-control implementation would build a selector
/// string like `format!("label[for=\"{id}\"]")` out of the control's own
/// `id` attribute value — but that attribute value is live page content
/// (untrusted input in this repo's threat model), and could itself contain
/// `"` or other CSS-selector metacharacters, turning a lookup into either a
/// broken query or a selector-injection-shaped bug. Building this map once
/// via direct attribute comparison (never selector-text interpolation)
/// sidesteps that class of bug entirely, and is also strictly more
/// efficient (one document walk total instead of one per control).
fn build_label_map(document: &Document) -> Result<HashMap<String, ElementHandle>, SemanticError> {
    let mut map = HashMap::new();
    walk_document_order(document, |handle| {
        if let Ok(element) = document.as_element(handle) {
            if document.tag_name(element)? == "label" {
                if let Some(for_value) = document.attribute(element, "for")? {
                    map.entry(for_value.to_string()).or_insert(element);
                }
            }
        }
        Ok(())
    })?;
    Ok(map)
}

/// Walks upward from `start`'s parent, bounded by
/// [`MAX_ANCESTOR_LOOKUP_WALK`], returning the nearest `<form>` ancestor
/// element, if any.
fn nearest_form_ancestor(
    document: &Document,
    start: NodeHandle,
) -> Result<Option<ElementHandle>, SemanticError> {
    let mut current = document.node(start)?.parent();
    let mut steps = 0;
    while let Some(handle) = current {
        steps += 1;
        if steps > MAX_ANCESTOR_LOOKUP_WALK {
            break;
        }
        if let Ok(element) = document.as_element(handle) {
            if document.tag_name(element)? == "form" {
                return Ok(Some(element));
            }
        }
        current = document.node(handle)?.parent();
    }
    Ok(None)
}

fn build_forms(
    document: &Document,
    form_handles: &[ElementHandle],
    roles: &[SemanticNode],
    for_map: &HashMap<String, ElementHandle>,
) -> Result<Vec<FormNode>, SemanticError> {
    let mut forms = Vec::with_capacity(form_handles.len());
    let mut form_index = HashMap::with_capacity(form_handles.len());
    for (index, &handle) in form_handles.iter().enumerate() {
        forms.push(FormNode {
            handle,
            accessible_name: derive_accessible_name(document, handle, for_map)?,
            action: document.attribute(handle, "action")?.map(str::to_string),
            method: document.attribute(handle, "method")?.map(str::to_string),
            controls: Vec::new(),
        });
        form_index.insert(handle, index);
    }
    if forms.is_empty() {
        return Ok(forms);
    }

    for candidate in roles {
        let tag = document.tag_name(candidate.handle)?;
        if !matches!(tag, "input" | "select" | "textarea" | "button") {
            continue;
        }
        if let Some(form_handle) = nearest_form_ancestor(document, candidate.handle.node_handle())?
        {
            if let Some(&index) = form_index.get(&form_handle) {
                forms[index].controls.push(candidate.handle);
            }
        }
    }

    Ok(forms)
}

/// Extracts a full [`SemanticIndex`] snapshot for `document`, computed fresh
/// against `document`'s current [`Revision`] (see the module docs above for
/// the two-pass shape and the `machina_selectors` integration).
pub fn extract_semantic_index(document: &Document) -> Result<SemanticIndex, SemanticError> {
    let for_map = build_label_map(document)?;
    let candidates = candidate_selector()?;

    let mut roles = Vec::new();
    let mut headings = Vec::new();
    let mut links = Vec::new();
    let mut interactive_elements = Vec::new();
    let mut form_handles = Vec::new();
    let mut truncated = false;

    walk_document_order(document, |handle| {
        let Ok(element) = document.as_element(handle) else {
            return Ok(());
        };
        if !matches_compiled(document, element, &candidates)? {
            return Ok(());
        }
        let tag = document.tag_name(element)?;
        if tag == "form" {
            if form_handles.len() < MAX_SEMANTIC_ITEMS {
                form_handles.push(element);
            } else {
                truncated = true;
            }
        }

        let Some(role) = derive_role(document, element)? else {
            return Ok(());
        };
        let accessible_name = derive_accessible_name(document, element, &for_map)?;
        let level = heading_level(tag);
        let node = SemanticNode {
            handle: element,
            role: role.clone(),
            accessible_name: accessible_name.clone(),
            heading_level: level,
        };

        if roles.len() < MAX_SEMANTIC_ITEMS {
            roles.push(node.clone());
        } else {
            truncated = true;
        }

        if level.is_some() {
            if headings.len() < MAX_SEMANTIC_ITEMS {
                headings.push(node.clone());
            } else {
                truncated = true;
            }
        }

        if role == "link" {
            let href = document
                .attribute(element, "href")?
                .unwrap_or("")
                .to_string();
            if links.len() < MAX_SEMANTIC_ITEMS {
                links.push(LinkNode {
                    handle: element,
                    accessible_name: accessible_name.clone(),
                    href,
                });
            } else {
                truncated = true;
            }
        }

        if INTERACTIVE_ROLES.contains(&role.as_str()) {
            if interactive_elements.len() < MAX_SEMANTIC_ITEMS {
                interactive_elements.push(node);
            } else {
                truncated = true;
            }
        }

        Ok(())
    })?;

    let forms = build_forms(document, &form_handles, &roles, &for_map)?;

    Ok(SemanticIndex {
        revision: document.revision(),
        roles,
        headings,
        links,
        forms,
        interactive_elements,
        truncated,
    })
}
