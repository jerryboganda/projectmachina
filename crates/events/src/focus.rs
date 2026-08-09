//! The active-element/focus model, folded into
//! [`crate::target::EventTargetRegistry`] (shares its construction and
//! lifecycle hooks — see `EventTargetRegistry::handle_node_changed`).
//!
//! **Minimal focusability** (deliberately not spec-complete): a parseable
//! `tabindex` attribute (any integer, including negative — negative
//! `tabindex` is still programmatically focusable, just excluded from
//! sequential Tab navigation, which this crate does not implement at all)
//! OR one of a small built-in tag set (`a[href]`, `button`, `input`,
//! `select`, `textarea`, `area[href]`, `iframe`). Visibility, geometry,
//! `disabled`, and `pointer-events` are explicit, out-of-scope gaps that
//! need M3-T12 layout/style information this crate does not have.

use machina_dom::{Document, DomError, NodeHandle, MAX_ANCESTOR_WALK};

use crate::dispatch::dispatch_event;
use crate::error::EventError;
use crate::event::{EventInit, EventKind};
use crate::target::EventTargetRegistry;

/// The result of a [`focus`]/[`blur`] call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusOutcome {
    /// The requested element was already the active element (`focus`) or
    /// there was no active element to clear (`blur`). No events fired.
    NoOp,
    /// Focus moved; `previous` is whatever was active before this call (if
    /// anything).
    Changed { previous: Option<NodeHandle> },
}

/// `true` if `handle` currently resolves to an `Element` that qualifies
/// under this crate's minimal focusability rule. `false` (not an error) for
/// a resolvable non-`Element` node; propagates a [`DomError`] only for a
/// handle that does not resolve at all.
pub fn is_focusable_by_default(
    document: &Document,
    handle: NodeHandle,
) -> Result<bool, EventError> {
    let element = match document.as_element(handle) {
        Ok(element) => element,
        Err(DomError::WrongKind) => return Ok(false),
        Err(err) => return Err(EventError::from(err)),
    };
    if let Some(tabindex) = document.attribute(element, "tabindex")? {
        if tabindex.trim().parse::<i64>().is_ok() {
            return Ok(true);
        }
    }
    let tag = document.tag_name(element)?;
    let focusable = match tag {
        "button" | "input" | "select" | "textarea" | "iframe" => true,
        "a" | "area" => document.attribute(element, "href")?.is_some(),
        _ => false,
    };
    Ok(focusable)
}

/// `start` itself if focusable, else the nearest ancestor (walking up
/// through `.parent()`, bounded by `machina_dom::MAX_ANCESTOR_WALK`) that
/// qualifies under [`is_focusable_by_default`]. `Ok(None)` if neither `start`
/// nor any ancestor qualifies.
pub fn nearest_focusable(
    document: &Document,
    start: NodeHandle,
) -> Result<Option<NodeHandle>, EventError> {
    let mut current = Some(start);
    let mut steps = 0usize;
    while let Some(handle) = current {
        if is_focusable_by_default(document, handle)? {
            return Ok(Some(handle));
        }
        steps += 1;
        if steps > MAX_ANCESTOR_WALK {
            return Err(EventError::from(DomError::DepthLimitExceeded));
        }
        current = document.node(handle)?.parent();
    }
    Ok(None)
}

/// Moves focus to `target`.
///
/// No-op (no events fired) if `target` is already the active element.
/// Otherwise sets the active element to `target` **before** firing any
/// event (deliberate — matches observable Chromium behavior; exact WPT
/// edge-case ordering is explicitly deferred past this task), then fires
/// `blur`+`focusout` at the previous active element (skipped if it no
/// longer resolves — already detached/freed), then `focus`+`focusin` at
/// `target`.
pub fn focus(
    document: &mut Document,
    registry: &mut EventTargetRegistry,
    target: NodeHandle,
) -> Result<FocusOutcome, EventError> {
    document
        .node(target)
        .map_err(|_| EventError::TargetNotFound)?;
    if registry.active_element() == Some(target) {
        return Ok(FocusOutcome::NoOp);
    }
    let previous = registry.active_element();
    registry.set_active_element(Some(target));

    if let Some(previous) = previous {
        if document.node(previous).is_ok() {
            dispatch_event(
                document,
                registry,
                previous,
                EventKind::Blur,
                EventInit::for_kind(EventKind::Blur),
            )?;
            dispatch_event(
                document,
                registry,
                previous,
                EventKind::FocusOut,
                EventInit::for_kind(EventKind::FocusOut),
            )?;
        }
    }

    dispatch_event(
        document,
        registry,
        target,
        EventKind::Focus,
        EventInit::for_kind(EventKind::Focus),
    )?;
    dispatch_event(
        document,
        registry,
        target,
        EventKind::FocusIn,
        EventInit::for_kind(EventKind::FocusIn),
    )?;

    Ok(FocusOutcome::Changed { previous })
}

/// Clears focus unconditionally (no "fall back to body" — explicitly
/// deferred, non-breaking to add later).
///
/// No-op (no events) if nothing is currently focused. Otherwise fires
/// `blur`+`focusout` at the (still-active, about-to-be-cleared) active
/// element, then sets the active element to `None`.
pub fn blur(
    document: &mut Document,
    registry: &mut EventTargetRegistry,
) -> Result<FocusOutcome, EventError> {
    let Some(previous) = registry.active_element() else {
        return Ok(FocusOutcome::NoOp);
    };

    if document.node(previous).is_ok() {
        dispatch_event(
            document,
            registry,
            previous,
            EventKind::Blur,
            EventInit::for_kind(EventKind::Blur),
        )?;
        dispatch_event(
            document,
            registry,
            previous,
            EventKind::FocusOut,
            EventInit::for_kind(EventKind::FocusOut),
        )?;
    }
    registry.set_active_element(None);

    Ok(FocusOutcome::Changed {
        previous: Some(previous),
    })
}
