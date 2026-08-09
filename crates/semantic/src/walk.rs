//! Shared bounded, iterative (never recursive) pre-order document-order DFS
//! walk, used by both `extract.rs` (the id/label-map pass and the main
//! semantic-index pass) and `metadata.rs`. Reads directly from
//! `Document::children`/`Document::node` — no cloning, no parallel tree.
//! Mirrors `machina_selectors::query::walk_document_order`'s exact shape
//! (that function is crate-private to `machina-selectors`, so this crate
//! keeps its own copy rather than taking a dependency on a private helper).

use machina_dom::{Document, NodeHandle};

use crate::error::SemanticError;
use crate::limits::{LimitKind, MAX_TOTAL_NODES_VISITED};

pub(crate) fn walk_document_order<F>(document: &Document, mut visit: F) -> Result<(), SemanticError>
where
    F: FnMut(NodeHandle) -> Result<(), SemanticError>,
{
    let mut stack = vec![document.root()];
    let mut visited: u64 = 0;
    while let Some(handle) = stack.pop() {
        visited += 1;
        if visited > MAX_TOTAL_NODES_VISITED {
            return Err(SemanticError::TooComplex {
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
