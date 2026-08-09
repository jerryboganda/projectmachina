//! Right-to-left, bounded-backtracking selector matcher (design §2). A
//! standard production-engine approach (Blink/WebKit/Servo all use a
//! variant of this), reimplemented clean-room against `machina_dom`'s real
//! API per D01-B — no external selector-engine crate is used or vendored.

use machina_dom::{Document, ElementHandle, NodeHandle};

use crate::css::ast::{
    AttrOperator, Combinator, ComplexSelector, CompoundSelector, PseudoClass, SelectorList,
    SimpleSelector,
};
use crate::error::{LimitKind, QueryError};
use crate::limits::MAX_WALK_STEPS;

/// `true` if `element` matches any member of `list` (comma = logical OR).
pub(crate) fn matches_list(
    document: &Document,
    element: ElementHandle,
    list: &SelectorList,
) -> Result<bool, QueryError> {
    for complex in &list.selectors {
        if matches_complex(document, element, complex)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn matches_complex(
    document: &Document,
    element: ElementHandle,
    complex: &ComplexSelector,
) -> Result<bool, QueryError> {
    match_at(document, element, complex, complex.compounds.len() - 1)
}

/// Tests `complex.compounds[idx]` against `element`, then — on success —
/// walks left per `complex.combinators[idx - 1]` toward `idx - 1`,
/// backtracking through every candidate the combinator offers (design §2,
/// step 3) rather than stopping at the first one that fails the rest of the
/// chain.
fn match_at(
    document: &Document,
    element: ElementHandle,
    complex: &ComplexSelector,
    idx: usize,
) -> Result<bool, QueryError> {
    if !match_compound(document, element, &complex.compounds[idx])? {
        return Ok(false);
    }
    if idx == 0 {
        return Ok(true);
    }
    let combinator = complex.combinators[idx - 1];
    match combinator {
        Combinator::Descendant => {
            let mut current = document.node(element.node_handle())?.parent();
            let mut steps = 0usize;
            while let Some(handle) = current {
                steps += 1;
                if steps > MAX_WALK_STEPS {
                    return Err(QueryError::TooComplex {
                        limit: LimitKind::AncestorWalk,
                    });
                }
                if let Ok(ancestor) = document.as_element(handle) {
                    if match_at(document, ancestor, complex, idx - 1)? {
                        return Ok(true);
                    }
                }
                current = document.node(handle)?.parent();
            }
            Ok(false)
        }
        Combinator::Child => {
            let parent = document.node(element.node_handle())?.parent();
            match parent {
                Some(handle) => match document.as_element(handle) {
                    Ok(ancestor) => match_at(document, ancestor, complex, idx - 1),
                    Err(_) => Ok(false),
                },
                None => Ok(false),
            }
        }
        Combinator::AdjacentSibling => {
            match previous_element_sibling(document, element.node_handle())? {
                Some(sibling) => match_at(document, sibling, complex, idx - 1),
                None => Ok(false),
            }
        }
        Combinator::GeneralSibling => {
            let mut current = element.node_handle();
            let mut steps = 0usize;
            loop {
                steps += 1;
                if steps > MAX_WALK_STEPS {
                    return Err(QueryError::TooComplex {
                        limit: LimitKind::SiblingWalk,
                    });
                }
                match previous_element_sibling(document, current)? {
                    Some(sibling) => {
                        if match_at(document, sibling, complex, idx - 1)? {
                            return Ok(true);
                        }
                        current = sibling.node_handle();
                    }
                    None => return Ok(false),
                }
            }
        }
    }
}

/// Walks `previous_sibling` links (skipping non-`Element` nodes: text and
/// comment siblings do not count as CSS siblings) to find the nearest
/// preceding element sibling, bounded by [`MAX_WALK_STEPS`].
fn previous_element_sibling(
    document: &Document,
    handle: NodeHandle,
) -> Result<Option<ElementHandle>, QueryError> {
    let mut current = document.node(handle)?.previous_sibling();
    let mut steps = 0usize;
    while let Some(sibling) = current {
        steps += 1;
        if steps > MAX_WALK_STEPS {
            return Err(QueryError::TooComplex {
                limit: LimitKind::SiblingWalk,
            });
        }
        if let Ok(element) = document.as_element(sibling) {
            return Ok(Some(element));
        }
        current = document.node(sibling)?.previous_sibling();
    }
    Ok(None)
}

fn next_element_sibling(
    document: &Document,
    handle: NodeHandle,
) -> Result<Option<ElementHandle>, QueryError> {
    let mut current = document.node(handle)?.next_sibling();
    let mut steps = 0usize;
    while let Some(sibling) = current {
        steps += 1;
        if steps > MAX_WALK_STEPS {
            return Err(QueryError::TooComplex {
                limit: LimitKind::SiblingWalk,
            });
        }
        if let Ok(element) = document.as_element(sibling) {
            return Ok(Some(element));
        }
        current = document.node(sibling)?.next_sibling();
    }
    Ok(None)
}

/// 1-based position of `element` among its parent's `Element` children
/// (text/comment siblings do not count), counting from the start (for
/// `:nth-child`) or from the end (for `:nth-last-child`, `from_end = true`).
fn element_sibling_position(
    document: &Document,
    element: ElementHandle,
    from_end: bool,
) -> Result<i64, QueryError> {
    let mut position: i64 = 1;
    let mut current = element.node_handle();
    let mut steps = 0usize;
    loop {
        steps += 1;
        if steps > MAX_WALK_STEPS {
            return Err(QueryError::TooComplex {
                limit: LimitKind::SiblingWalk,
            });
        }
        let next = if from_end {
            next_element_sibling(document, current)?
        } else {
            previous_element_sibling(document, current)?
        };
        match next {
            Some(sibling) => {
                position += 1;
                current = sibling.node_handle();
            }
            None => return Ok(position),
        }
    }
}

pub(crate) fn match_compound(
    document: &Document,
    element: ElementHandle,
    compound: &CompoundSelector,
) -> Result<bool, QueryError> {
    for simple in &compound.simple_selectors {
        if !match_simple(document, element, simple)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn attribute_value_words(value: &str) -> impl Iterator<Item = &str> {
    value.split_ascii_whitespace()
}

fn match_simple(
    document: &Document,
    element: ElementHandle,
    simple: &SimpleSelector,
) -> Result<bool, QueryError> {
    match simple {
        SimpleSelector::Universal => Ok(true),
        SimpleSelector::Type(name) => Ok(document.tag_name(element)? == name.as_str()),
        SimpleSelector::Id(id) => Ok(document
            .attribute(element, "id")?
            .map(|v| v == id.as_str())
            .unwrap_or(false)),
        SimpleSelector::Class(class) => {
            let value = document.attribute(element, "class")?;
            Ok(value
                .map(|v| attribute_value_words(v).any(|word| word == class.as_str()))
                .unwrap_or(false))
        }
        SimpleSelector::Attribute { name, operator } => {
            let value = document.attribute(element, name)?;
            match (operator, value) {
                (AttrOperator::Present, Some(_)) => Ok(true),
                (AttrOperator::Present, None) => Ok(false),
                (AttrOperator::Equals(expected), Some(v)) => Ok(v == expected.as_str()),
                (AttrOperator::Includes(expected), Some(v)) => {
                    Ok(attribute_value_words(v).any(|word| word == expected.as_str()))
                }
                (AttrOperator::DashMatch(expected), Some(v)) => {
                    Ok(v == expected.as_str() || v.starts_with(&format!("{expected}-")))
                }
                (AttrOperator::PrefixMatch(expected), Some(v)) => {
                    Ok(!expected.is_empty() && v.starts_with(expected.as_str()))
                }
                (AttrOperator::SuffixMatch(expected), Some(v)) => {
                    Ok(!expected.is_empty() && v.ends_with(expected.as_str()))
                }
                (AttrOperator::SubstringMatch(expected), Some(v)) => {
                    Ok(!expected.is_empty() && v.contains(expected.as_str()))
                }
                (_, None) => Ok(false),
            }
        }
        SimpleSelector::Negation(inner) => Ok(!match_compound(document, element, inner)?),
        SimpleSelector::PseudoClass(pseudo) => match_pseudo(document, element, pseudo),
    }
}

fn match_pseudo(
    document: &Document,
    element: ElementHandle,
    pseudo: &PseudoClass,
) -> Result<bool, QueryError> {
    match pseudo {
        PseudoClass::FirstChild => {
            Ok(previous_element_sibling(document, element.node_handle())?.is_none())
        }
        PseudoClass::LastChild => {
            Ok(next_element_sibling(document, element.node_handle())?.is_none())
        }
        PseudoClass::OnlyChild => Ok(previous_element_sibling(document, element.node_handle())?
            .is_none()
            && next_element_sibling(document, element.node_handle())?.is_none()),
        PseudoClass::Empty => Ok(document.children(element.node_handle())?.is_empty()),
        PseudoClass::Root => {
            Ok(document.node(element.node_handle())?.parent() == Some(document.root()))
        }
        PseudoClass::NthChild(expr) => {
            let position = element_sibling_position(document, element, false)?;
            Ok(expr.matches(position))
        }
        PseudoClass::NthLastChild(expr) => {
            let position = element_sibling_position(document, element, true)?;
            Ok(expr.matches(position))
        }
    }
}
