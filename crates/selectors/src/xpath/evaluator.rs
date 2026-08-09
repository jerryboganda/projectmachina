//! XPath evaluator: walks the AST produced by [`crate::xpath::parser`]
//! against a live `Document`, axis by axis, predicate by predicate,
//! producing an ordered, deduplicated node-set (design §5 — no boolean/
//! number/string top-level coercion).

use std::collections::HashSet;

use machina_dom::{Document, NodeHandle, NodeKind};

use crate::error::{LimitKind, QueryError};
use crate::limits::MAX_TOTAL_NODES_VISITED;
use crate::xpath::ast::{Axis, NodeTest, Predicate, Step, XPathExpr};
use crate::xpath::parser::parse;

/// One item in an XPath result node-set. Node items are ordinary DOM
/// handles; attribute items are a distinct variant (design §5's
/// "attribute-axis result gap") since `machina_dom`'s `AttributeMap` is an
/// inline `Vec<(Atom, Box<str>)>`, not arena nodes — an attribute has no
/// `NodeHandle` to hand back.
#[derive(Clone, Debug, PartialEq)]
pub enum XPathItem {
    Node(NodeHandle),
    Attribute {
        owner: machina_dom::ElementHandle,
        name: String,
        value: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct XPathResult {
    pub revision: machina_dom::Revision,
    pub items: Vec<XPathItem>,
}

/// Evaluates `expr_text` against `document`. Absolute paths ignore
/// `context`; relative paths with `context: None` fail with
/// [`QueryError::ContextNodeRequired`] rather than silently defaulting to
/// the document root (design §5).
pub fn evaluate_xpath(
    document: &Document,
    expr_text: &str,
    context: Option<NodeHandle>,
) -> Result<XPathResult, QueryError> {
    let expr: XPathExpr = parse(expr_text)?;
    let start: Vec<XPathItem> = if expr.absolute {
        vec![XPathItem::Node(document.root())]
    } else {
        match context {
            Some(handle) => vec![XPathItem::Node(handle)],
            None => return Err(QueryError::ContextNodeRequired),
        }
    };

    let mut current = start;
    for step in &expr.steps {
        current = evaluate_step(document, &current, step)?;
    }

    Ok(XPathResult {
        revision: document.revision(),
        items: current,
    })
}

fn evaluate_step(
    document: &Document,
    contexts: &[XPathItem],
    step: &Step,
) -> Result<Vec<XPathItem>, QueryError> {
    let mut merged = Vec::new();
    for context_item in contexts {
        let mut candidates = axis_expand(document, context_item, step.axis, &step.test)?;
        for predicate in &step.predicates {
            candidates = filter_by_predicate(document, &candidates, predicate)?;
        }
        merged.extend(candidates);
    }
    Ok(dedupe_preserving_order(merged))
}

fn dedupe_preserving_order(items: Vec<XPathItem>) -> Vec<XPathItem> {
    let mut seen_nodes: HashSet<NodeHandle> = HashSet::new();
    let mut seen_attrs: HashSet<(NodeHandle, String)> = HashSet::new();
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match &item {
            XPathItem::Node(handle) => {
                if seen_nodes.insert(*handle) {
                    out.push(item);
                }
            }
            XPathItem::Attribute { owner, name, .. } => {
                let key = (owner.node_handle(), name.clone());
                if seen_attrs.insert(key) {
                    out.push(item);
                }
            }
        }
    }
    out
}

fn axis_expand(
    document: &Document,
    context_item: &XPathItem,
    axis: Axis,
    test: &NodeTest,
) -> Result<Vec<XPathItem>, QueryError> {
    let handle = match context_item {
        XPathItem::Node(handle) => *handle,
        // Attribute nodes have no children/attributes/self of their own in
        // this crate's model; stepping further from one is a legitimate
        // empty result, not an error (spec-consistent: attribute nodes have
        // no children).
        XPathItem::Attribute { .. } => return Ok(Vec::new()),
    };

    match axis {
        Axis::Child => {
            let children = document.children(handle)?;
            Ok(children
                .into_iter()
                .filter(|child| node_test_matches(document, *child, test).unwrap_or(false))
                .map(XPathItem::Node)
                .collect())
        }
        Axis::Descendant => {
            let mut result = Vec::new();
            let mut stack: Vec<NodeHandle> = document.children(handle)?.into_iter().rev().collect();
            let mut visited: u64 = 0;
            while let Some(current) = stack.pop() {
                visited += 1;
                if visited > MAX_TOTAL_NODES_VISITED {
                    return Err(QueryError::TooComplex {
                        limit: LimitKind::TotalNodesVisited,
                    });
                }
                if node_test_matches(document, current, test)? {
                    result.push(XPathItem::Node(current));
                }
                let children = document.children(current)?;
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
            }
            Ok(result)
        }
        Axis::Attribute => {
            let name = match test {
                NodeTest::Name(name) => name.clone(),
                NodeTest::Any => {
                    return Err(QueryError::UnsupportedFeature {
                        feature: "@* (wildcard attribute node test)".to_string(),
                        position: 0,
                    });
                }
                NodeTest::Text | NodeTest::Node | NodeTest::Comment => return Ok(Vec::new()),
            };
            match document.as_element(handle) {
                Ok(element) => match document.attribute(element, &name)? {
                    Some(value) => Ok(vec![XPathItem::Attribute {
                        owner: element,
                        name,
                        value: value.to_string(),
                    }]),
                    None => Ok(Vec::new()),
                },
                Err(_) => Ok(Vec::new()),
            }
        }
        Axis::SelfAxis => {
            if node_test_matches(document, handle, test)? {
                Ok(vec![XPathItem::Node(handle)])
            } else {
                Ok(Vec::new())
            }
        }
        Axis::Parent => match document.node(handle)?.parent() {
            Some(parent) if node_test_matches(document, parent, test)? => {
                Ok(vec![XPathItem::Node(parent)])
            }
            _ => Ok(Vec::new()),
        },
    }
}

fn node_test_matches(
    document: &Document,
    handle: NodeHandle,
    test: &NodeTest,
) -> Result<bool, QueryError> {
    let kind = document.node(handle)?.kind();
    Ok(match test {
        NodeTest::Any => kind == NodeKind::Element,
        NodeTest::Name(name) => {
            kind == NodeKind::Element
                && document
                    .as_element(handle)
                    .ok()
                    .and_then(|element| document.tag_name(element).ok().map(|t| t == name.as_str()))
                    .unwrap_or(false)
        }
        NodeTest::Text => kind == NodeKind::Text,
        NodeTest::Comment => kind == NodeKind::Comment,
        NodeTest::Node => true,
    })
}

fn filter_by_predicate(
    document: &Document,
    items: &[XPathItem],
    predicate: &Predicate,
) -> Result<Vec<XPathItem>, QueryError> {
    match predicate {
        Predicate::Index(index) => Ok(if *index >= 1 && *index <= items.len() {
            vec![items[*index - 1].clone()]
        } else {
            Vec::new()
        }),
        Predicate::Last => Ok(items.last().cloned().into_iter().collect()),
        Predicate::AttributeExists(name) => {
            let mut out = Vec::new();
            for item in items {
                if item_has_attribute(document, item, name)?.is_some() {
                    out.push(item.clone());
                }
            }
            Ok(out)
        }
        Predicate::AttributeEquals(name, expected) => {
            let mut out = Vec::new();
            for item in items {
                if let Some(value) = item_has_attribute(document, item, name)? {
                    if value == *expected {
                        out.push(item.clone());
                    }
                }
            }
            Ok(out)
        }
        Predicate::And(left, right) => {
            let left_filtered = filter_by_predicate(document, items, left)?;
            filter_by_predicate(document, &left_filtered, right)
        }
    }
}

/// `Some(value)` if `item` is an element node carrying attribute `name`;
/// `None` for a non-element node or an XPath attribute-item leaf (which has
/// no attributes of its own) — a defined, narrow behavior rather than an
/// error, since checking "does this attribute have an attribute" is
/// meaningless but not malformed.
fn item_has_attribute(
    document: &Document,
    item: &XPathItem,
    name: &str,
) -> Result<Option<String>, QueryError> {
    match item {
        XPathItem::Node(handle) => match document.as_element(*handle) {
            Ok(element) => Ok(document.attribute(element, name)?.map(|v| v.to_string())),
            Err(_) => Ok(None),
        },
        XPathItem::Attribute { .. } => Ok(None),
    }
}
