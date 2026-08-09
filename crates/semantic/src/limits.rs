//! Bounded-output and bounded-walk limits for every extraction operation in
//! this crate. "Outputs are bounded/paginated and do not require duplicate
//! DOM copies" (M2-T13 acceptance criterion) is enforced here, and every
//! call site that hits one of these bounds sets a `truncated: bool` flag on
//! its result rather than silently dropping data or erroring on a
//! legitimately large real document.

use std::fmt;

/// Defensive backstop on total nodes visited during one full-document DFS
/// (id-map build pass, semantic-index build pass). Reuses
/// `machina_selectors::limits::MAX_TOTAL_NODES_VISITED` directly rather than
/// mirroring a second copy of the numeric value, so the two crates cannot
/// silently drift apart — same coordination pattern `crates/selectors`
/// itself used for `machina_dom::MAX_ANCESTOR_WALK`. Deliberately generous:
/// a legitimate large document can easily exceed 100_000 total nodes: this
/// exists only to fail closed against a corrupted/cyclic tree state that
/// should be structurally impossible per `machina_dom`'s own invariants.
pub const MAX_TOTAL_NODES_VISITED: u64 = machina_selectors::limits::MAX_TOTAL_NODES_VISITED;

/// Bound on how many ancestors a single upward-parent-chain lookup will walk
/// before giving up: used both for a control's "wrapping `<label>`"
/// accessible-name fallback (`<label>Name <input></label>`) and for finding
/// a form control's nearest enclosing `<form>` ancestor. Small and local:
/// real HTML never nests a form control more than a handful of levels deep
/// inside its own label or owning form.
pub const MAX_ANCESTOR_LOOKUP_WALK: usize = 64;

/// Bound (in `char`s, not bytes) on one computed accessible name. Prevents
/// an adversarial document (for example a single `<button>` wrapping
/// megabytes of nested text) from producing an unbounded accessible name.
/// Chosen generously above any realistic accessible name (typical accessible
/// names are a few words to a sentence).
pub const MAX_ACCESSIBLE_NAME_CHARS: usize = 2_000;

/// Bound (in `char`s) on the extracted `<title>` text and on each individual
/// `<meta>` tag's extracted value.
pub const MAX_METADATA_VALUE_CHARS: usize = 2_000;

/// Bound on how many entries `SemanticIndex::roles` (and therefore also its
/// `headings`/`links`/`forms`/`interactive_elements` subsets, plus
/// `DocumentMetadata::meta`) may carry. Exceeding this sets `truncated =
/// true` and stops adding further items to the relevant vector(s); the walk
/// itself still completes (so `id`/`for` resolution for items already
/// collected stays correct), it just stops accumulating output past this
/// cap.
pub const MAX_SEMANTIC_ITEMS: usize = 10_000;

/// Bound on how many nodes one bounded text-content collection walk (used
/// for accessible-name derivation and `<title>` extraction, `text.rs`) will
/// visit before giving up and truncating. Deliberately smaller/local rather
/// than [`MAX_TOTAL_NODES_VISITED`]: this walk runs once per element that
/// needs a name (potentially many times per document), and unlike the
/// whole-document passes, hitting this cap does not indicate a corrupted
/// tree — it just means "stop collecting text for this one name," which is
/// truncation, not an error (see `text::collect_text_content`'s docs).
pub const MAX_TEXT_WALK_NODES: u64 = 50_000;

/// Bound (in bytes) on generated markdown output. Exceeding this truncates
/// at the nearest safe `char` boundary, appends a truncation marker, and
/// sets `MarkdownDocument::truncated = true`; markdown generation for that
/// document stops at that point (deterministic, not resumable within a
/// single call — see `MarkdownDocument`'s own docs for the disclosed
/// pagination gap).
pub const MAX_MARKDOWN_BYTES: usize = 200_000;

/// Which bounded-walk guard was hit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LimitKind {
    /// The full-document DFS (id-map build pass or semantic-index build
    /// pass) exceeded [`MAX_TOTAL_NODES_VISITED`].
    TotalNodesVisited,
    /// The markdown-generation DFS exceeded [`MAX_TOTAL_NODES_VISITED`]
    /// (distinct backstop instance from the semantic-index walk, same
    /// bound).
    MarkdownWalk,
}

impl fmt::Display for LimitKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TotalNodesVisited => formatter.write_str("total nodes visited"),
            Self::MarkdownWalk => formatter.write_str("markdown generation walk"),
        }
    }
}
