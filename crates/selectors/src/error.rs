//! Typed failure modes for every selector/XPath operation. No parser or
//! matcher path in this crate panics on caller-reachable input (malformed
//! selector/XPath text, adversarial nesting, stale/cross-document handles);
//! every such condition maps to one of these variants instead.
//!
//! Three-way split, enforced throughout `css/` and `xpath/`:
//! - a **legitimate empty match** is `Ok(QueryResult { elements: vec![], .. })`,
//!   never an error;
//! - **malformed syntax** is always [`QueryError::InvalidSelector`] /
//!   [`QueryError::InvalidXPath`], produced strictly during parsing, before
//!   any tree walk begins (a parse error can never "partially match");
//! - a **valid-but-out-of-scope construct** (for example `:hover` or the
//!   `following-sibling::` axis) is always [`QueryError::UnsupportedFeature`],
//!   distinct from both of the above — this is what prevents the most
//!   dangerous silent-failure mode for an automation product: a selector
//!   that silently dropped an unsupported clause and matched something else
//!   than the caller intended.

use std::fmt;

use machina_dom::DomError;

/// Which bounded-walk guard was hit. Mirrors `machina_dom`'s
/// `MAX_ANCESTOR_WALK` posture: adversarially deep/wide trees or pathological
/// selector/XPath nesting fail closed with a typed error, never unbounded
/// work, unbounded recursion, or a stack overflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LimitKind {
    /// A descendant-combinator or descendant-axis ancestor/DFS walk exceeded
    /// `machina_dom::MAX_ANCESTOR_WALK` steps.
    AncestorWalk,
    /// A general-sibling-combinator or sibling-position (`nth-child`) walk
    /// exceeded the same bound.
    SiblingWalk,
    /// Selector or XPath grammar nesting (for example `:not(:not(:not(...)))`
    /// or a path with an excessive number of steps) exceeded this crate's
    /// bounded nesting depth.
    SelectorNesting,
    /// A defensive backstop on total nodes visited during a document-order
    /// walk (`query_selector_all`, `descendant::`). Exists to fail closed
    /// against a corrupted/cyclic tree state that should be structurally
    /// impossible per `machina_dom`'s own invariants, not to reject
    /// legitimate large documents (the cap is generous).
    TotalNodesVisited,
}

impl fmt::Display for LimitKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AncestorWalk => formatter.write_str("ancestor walk"),
            Self::SiblingWalk => formatter.write_str("sibling walk"),
            Self::SelectorNesting => formatter.write_str("selector/xpath nesting depth"),
            Self::TotalNodesVisited => formatter.write_str("total nodes visited"),
        }
    }
}

/// Canonical error type for every selector/XPath entry point in this crate.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum QueryError {
    /// The CSS selector text could not be parsed: malformed syntax, staged
    /// strictly before any matching begins.
    InvalidSelector { message: String, position: usize },
    /// The XPath expression text could not be parsed: malformed syntax,
    /// staged strictly before any evaluation begins.
    InvalidXPath { message: String, position: usize },
    /// The selector/XPath text is syntactically well-formed but names a
    /// construct explicitly out of this crate's current scope (for example
    /// `:hover`, `:has()`, or the `following-sibling::` axis). Never treated
    /// as a silent no-op or a silent empty match.
    UnsupportedFeature { feature: String, position: usize },
    /// A bounded-walk guard (see [`LimitKind`]) was hit; the operation
    /// failed closed rather than doing unbounded work.
    TooComplex { limit: LimitKind },
    /// A relative XPath expression was evaluated with no context node. Never
    /// silently defaults to the document root, since that would mask a
    /// caller bug in per-element locator resolution.
    ContextNodeRequired,
    /// A DOM-layer failure (stale/cross-document/closed handle, wrong node
    /// kind, ...) surfaced while walking or resolving a node during
    /// matching/evaluation.
    DomError(DomError),
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelector { message, position } => {
                write!(formatter, "invalid selector at byte {position}: {message}")
            }
            Self::InvalidXPath { message, position } => {
                write!(formatter, "invalid xpath at byte {position}: {message}")
            }
            Self::UnsupportedFeature { feature, position } => write!(
                formatter,
                "unsupported (valid but out-of-scope) feature at byte {position}: {feature}"
            ),
            Self::TooComplex { limit } => write!(formatter, "too complex: {limit} bound exceeded"),
            Self::ContextNodeRequired => {
                formatter.write_str("relative xpath expression requires a context node")
            }
            Self::DomError(inner) => write!(formatter, "dom error: {inner}"),
        }
    }
}

impl std::error::Error for QueryError {}

impl From<DomError> for QueryError {
    fn from(value: DomError) -> Self {
        QueryError::DomError(value)
    }
}
