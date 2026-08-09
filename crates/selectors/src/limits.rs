//! Bounded-walk limits shared by the CSS matcher and the XPath evaluator.
//!
//! Coordination note (design doc §0): `machina_dom::MAX_ANCESTOR_WALK` is
//! `pub` (re-exported from `machina_dom`'s crate root) as of the merged
//! M2-T05, not `pub(crate)` as the design doc's coordination note
//! anticipated might still be needed — verified directly against
//! `crates/dom/src/lib.rs` (`pub use mutation::MAX_ANCESTOR_WALK;`) before
//! writing this file. This crate therefore reuses that exact constant for
//! its own ancestor/sibling walks instead of mirroring a second copy of the
//! numeric value, so the two crates can never silently drift apart.

/// Bound for a single descendant-combinator ancestor walk or a
/// general-sibling-combinator / `nth-child` sibling walk. Reuses
/// `machina_dom`'s own bound directly (see module docs) rather than
/// defining a second, potentially-drifting constant.
pub const MAX_WALK_STEPS: usize = machina_dom::MAX_ANCESTOR_WALK;

/// Bound on selector/XPath grammar nesting depth (`:not(:not(...))`
/// recursion depth, total XPath steps in one path expression). Chosen to be
/// generous for any realistic hand-written or generated selector while
/// still being a hard, documented ceiling against adversarial input —
/// mirrors `machina-html-tree-builder`'s `max_reprocess_hops` posture (a
/// structural backstop that legitimate fixtures should never hit).
pub const MAX_NESTING_DEPTH: usize = 64;

/// Defensive backstop on total nodes visited during one document-order DFS
/// (`query_selector_all`, `descendant::` axis expansion). Deliberately much
/// larger than `MAX_WALK_STEPS`: a legitimate large document can easily
/// exceed 100_000 total nodes, so this must not reject real documents — it
/// exists only to fail closed against a corrupted/cyclic tree state that
/// should be structurally impossible per `machina_dom`'s own invariants.
pub const MAX_TOTAL_NODES_VISITED: u64 = 20_000_000;
