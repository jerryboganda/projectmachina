//! Shared document-order DFS walk (design §3, §8) and the `Document`-facing
//! CSS query entry points. No incrementally-maintained index — a plain
//! pre-order tree walk testing each candidate via `css::matcher`, per the
//! design's deliberate no-indexing MVP decision (§3).

use machina_dom::{Document, ElementHandle, Revision};

use crate::css::matcher::matches_list;
use crate::css::parser::parse_selector_list;
use crate::error::{LimitKind, QueryError};
use crate::limits::MAX_TOTAL_NODES_VISITED;

/// A query outcome self-describing the [`Revision`] it was computed against
/// (design §4b). Staleness detection is plain
/// `result.revision != document.revision()` — never guessed or implicit.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryResult {
    pub revision: Revision,
    pub elements: Vec<ElementHandle>,
}

/// Iterative (explicit-stack, never recursive) pre-order document-order DFS
/// from `document.root()`, calling `visit` on every node handle. Mirrors
/// `machina_dom`'s own iterative-traversal posture (`clone_node`,
/// `collect_children`) so a pathologically deep tree cannot blow this
/// crate's call stack either. Bounded by [`MAX_TOTAL_NODES_VISITED`] as a
/// defensive backstop only (see that constant's docs) — never rejects a
/// realistic document.
fn walk_document_order<F>(document: &Document, mut visit: F) -> Result<(), QueryError>
where
    F: FnMut(machina_dom::NodeHandle) -> Result<(), QueryError>,
{
    let mut stack = vec![document.root()];
    let mut visited: u64 = 0;
    while let Some(handle) = stack.pop() {
        visited += 1;
        if visited > MAX_TOTAL_NODES_VISITED {
            return Err(QueryError::TooComplex {
                limit: LimitKind::TotalNodesVisited,
            });
        }
        visit(handle)?;
        let children = document.children(handle)?;
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    Ok(())
}

/// Parses `selector_text` and returns every matching element, in document
/// order, stamped with the [`Revision`] the walk was performed against.
pub fn query_selector_all(
    document: &Document,
    selector_text: &str,
) -> Result<QueryResult, QueryError> {
    let list = parse_selector_list(selector_text)?;
    let mut elements = Vec::new();
    walk_document_order(document, |handle| {
        if let Ok(element) = document.as_element(handle) {
            if matches_list(document, element, &list)? {
                elements.push(element);
            }
        }
        Ok(())
    })?;
    Ok(QueryResult {
        revision: document.revision(),
        elements,
    })
}

/// The first element (document order) matching `selector_text`, or `None`
/// on a legitimate empty match (not an error — see the design's three-way
/// error split, `error` module docs).
pub fn query_selector(
    document: &Document,
    selector_text: &str,
) -> Result<Option<ElementHandle>, QueryError> {
    let list = parse_selector_list(selector_text)?;
    let mut found = None;
    walk_document_order(document, |handle| {
        if found.is_some() {
            return Ok(());
        }
        if let Ok(element) = document.as_element(handle) {
            if matches_list(document, element, &list)? {
                found = Some(element);
            }
        }
        Ok(())
    })?;
    Ok(found)
}

/// `true` if `element` itself matches `selector_text` (does not search the
/// tree). Useful for callers checking one already-resolved element handle
/// (mirrors `Element.matches()`).
pub fn matches(
    document: &Document,
    element: ElementHandle,
    selector_text: &str,
) -> Result<bool, QueryError> {
    let list = parse_selector_list(selector_text)?;
    matches_list(document, element, &list)
}

/// Direct DOM-API-style lookups (design §3): reuse the same document-order
/// walk with a specialized single-predicate fast path instead of round-
/// tripping through the CSS parser/AST — `id`/`class`/`tag` values are raw
/// HTML attribute/tag strings, not CSS selector syntax, so this also avoids
/// misinterpreting a literal id/class value that happens to contain CSS
/// metacharacters.
pub fn get_element_by_id(
    document: &Document,
    id: &str,
) -> Result<Option<ElementHandle>, QueryError> {
    let mut found = None;
    walk_document_order(document, |handle| {
        if found.is_some() {
            return Ok(());
        }
        if let Ok(element) = document.as_element(handle) {
            if document
                .attribute(element, "id")?
                .map(|v| v == id)
                .unwrap_or(false)
            {
                found = Some(element);
            }
        }
        Ok(())
    })?;
    Ok(found)
}

pub fn get_elements_by_class_name(
    document: &Document,
    class: &str,
) -> Result<QueryResult, QueryError> {
    let mut elements = Vec::new();
    walk_document_order(document, |handle| {
        if let Ok(element) = document.as_element(handle) {
            let has_class = document
                .attribute(element, "class")?
                .map(|v| v.split_ascii_whitespace().any(|word| word == class))
                .unwrap_or(false);
            if has_class {
                elements.push(element);
            }
        }
        Ok(())
    })?;
    Ok(QueryResult {
        revision: document.revision(),
        elements,
    })
}

pub fn get_elements_by_tag_name(document: &Document, tag: &str) -> Result<QueryResult, QueryError> {
    let mut elements = Vec::new();
    walk_document_order(document, |handle| {
        if let Ok(element) = document.as_element(handle) {
            if document.tag_name(element)? == tag {
                elements.push(element);
            }
        }
        Ok(())
    })?;
    Ok(QueryResult {
        revision: document.revision(),
        elements,
    })
}
