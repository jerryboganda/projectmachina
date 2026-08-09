//! CSS selector query engine and initial XPath evaluator for Project
//! Machina's native DOM (`machina-dom`).
//!
//! Depends only on `machina-dom` + std (no protocol/command-model/command-
//! bus/capability crate — see the M2-T10 design doc §0). Hand-written
//! recursive-descent parsers (clean-room, no external selector/XPath
//! crate) with `Result` at every production: no `unsafe`, and no
//! `unwrap`/`expect` on any path reachable from caller-supplied selector or
//! XPath text.
//!
//! Three top-level surfaces:
//! - [`query`] — CSS `query_selector`/`query_selector_all` and the direct
//!   `getElementById`-style lookups, all returning a [`query::QueryResult`]
//!   self-stamped with the [`machina_dom::Revision`] the walk ran against.
//! - [`css`] — the selector grammar AST, parser, and matcher, exposed for
//!   callers that want to compile once and match many elements without
//!   re-parsing (design §4a: a compiled selector is `Document`-agnostic and
//!   safely cacheable).
//! - [`xpath`] — the XPath grammar subset: `evaluate_xpath` plus the AST.
//!
//! See `.agent-state/design/M2-T10-selectors-xpath-design.md` for the full
//! design rationale (grammar scope, matcher algorithm, cache-invalidation
//! contract, and the three-way error split).

#![forbid(unsafe_code)]

pub mod css;
pub mod error;
pub mod limits;
pub mod query;
pub mod xpath;

pub use error::{LimitKind, QueryError};
pub use query::{
    get_element_by_id, get_elements_by_class_name, get_elements_by_tag_name, matches,
    query_selector, query_selector_all, QueryResult,
};
pub use xpath::{evaluate_xpath, XPathItem, XPathResult};
