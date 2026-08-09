//! XPath parsing and evaluation (design §5).

pub(crate) mod ast;
mod evaluator;
pub(crate) mod parser;
mod tokenizer;

pub use ast::{Axis, NodeTest, Predicate, Step, XPathExpr};
pub use evaluator::{evaluate_xpath, XPathItem, XPathResult};
