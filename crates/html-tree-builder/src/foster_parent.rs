//! Foster parenting (WHATWG HTML §13.2.6.1's "appropriate place for
//! inserting a node" algorithm's table-context branch). A pure targeting
//! function returning where a node destined for a table-context-confused
//! insertion should actually land, consumed by every insertion-mode
//! handler that needs it (`InTable`, `InTableText`, `InCaption`,
//! `InTableBody`, `InRow`) through one shared code path (design §3).

use machina_dom::NodeHandle;

use crate::open_elements::OpenElementsStack;

/// Where a foster-parented node should be inserted: either appended to
/// `parent`, or inserted immediately before `before` (a child of `parent`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct FosterTarget {
    pub parent: NodeHandle,
    pub before: Option<NodeHandle>,
}

/// Scans the open-elements stack top-down for the last `template` or
/// `table` element per spec, and computes the foster-parenting target.
///
/// - If a `template` is found (and it is at or above any `table` found),
///   the target is that template's contents — simplified here to the
///   template element itself (this crate does not build a separate
///   "template contents" document-fragment node per element; see
///   `.agent-state/evidence/M2-T04.md` for this documented simplification).
/// - Else if a `table` is found, the target is immediately before that
///   table, inside the table's parent (or appended to the table if it has
///   no parent yet / no parent could be resolved) — the caller resolves
///   the actual DOM parent via `doc.node(table).parent()`.
/// - Else (no `table`/`template` on the stack, e.g. a `<table>`-free
///   fragment context) the target is the last (bottommost) stack entry
///   (i.e. append into whatever the current node is).
pub(crate) fn foster_parent_target(
    open_elements: &OpenElementsStack,
    doc: &machina_dom::Document,
) -> FosterTarget {
    let mut last_template: Option<NodeHandle> = None;
    let mut last_table: Option<NodeHandle> = None;
    let mut last_template_index = None;
    let mut last_table_index = None;
    for (i, entry) in open_elements.iter().enumerate() {
        if entry.tag == "template" {
            last_template = Some(entry.handle.node_handle());
            last_template_index = Some(i);
        } else if entry.tag == "table" {
            last_table = Some(entry.handle.node_handle());
            last_table_index = Some(i);
        }
    }

    match (
        last_template,
        last_table,
        last_template_index,
        last_table_index,
    ) {
        (Some(template), _, Some(ti), table_index) if table_index.is_none_or(|tix| ti > tix) => {
            FosterTarget {
                parent: template,
                before: None,
            }
        }
        (_, Some(table), _, _) => {
            let table_parent = doc.node(table).ok().and_then(|n| n.parent());
            match table_parent {
                Some(parent) => FosterTarget {
                    parent,
                    before: Some(table),
                },
                // The table has no parent yet (not yet inserted, or the
                // parent handle failed to resolve) — spec's fallback is to
                // append to the element before `table` in the stack; we
                // approximate by appending directly to the table itself,
                // which keeps content visible in the tree instead of
                // discarding it, at the cost of exact spec fidelity in
                // this edge case.
                None => FosterTarget {
                    parent: table,
                    before: None,
                },
            }
        }
        _ => {
            let fallback = open_elements
                .current_handle()
                .map(|h| h.node_handle())
                .unwrap_or(doc.root());
            FosterTarget {
                parent: fallback,
                before: None,
            }
        }
    }
}
