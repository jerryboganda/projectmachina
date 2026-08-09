//! CSS selector parsing and matching (design §1-§2, §6).

pub(crate) mod ast;
pub(crate) mod matcher;
pub(crate) mod parser;
pub(crate) mod pseudo;
mod tokenizer;

pub use ast::{
    AttrOperator, Combinator, ComplexSelector, CompoundSelector, NthExpr, PseudoClass,
    SelectorList, SimpleSelector,
};

use machina_dom::{Document, ElementHandle};

use crate::error::QueryError;

/// Parses `source` into a compiled, `Document`-agnostic [`SelectorList`]
/// once, so a caller matching many elements against the same selector text
/// does not have to re-run the parser for each one (design §4a).
pub fn parse_selector(source: &str) -> Result<SelectorList, QueryError> {
    parser::parse_selector_list(source)
}

/// `true` if `element` matches a selector list already compiled via
/// [`parse_selector`].
pub fn matches_compiled(
    document: &Document,
    element: ElementHandle,
    list: &SelectorList,
) -> Result<bool, QueryError> {
    matcher::matches_list(document, element, list)
}
