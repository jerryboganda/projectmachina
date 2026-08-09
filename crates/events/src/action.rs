//! Synthetic click action: the `interaction.click.v1`-facing entry point.
//!
//! `perform_click` takes an already-resolved [`NodeHandle`] — selector
//! resolution (and `ELEMENT_AMBIGUOUS`) happens entirely upstream, in
//! M2-T10's query API, before this crate is ever called. Mapping this
//! module's error/outcome types onto `CanonicalErrorCode` is the native-core
//! wiring layer's job, not this crate's: this module stays free of
//! `serde`/JSON and returns typed Rust structs only.

use machina_dom::NodeHandle;

use crate::dispatch::{dispatch_event, is_attached};
use crate::error::EventError;
use crate::event::{EventDetail, EventInit, EventKind};
use crate::focus::{focus, nearest_focusable};
use crate::mouse::MouseEventInit;
use crate::target::EventTargetRegistry;

/// Whether the full click action ran to completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PostconditionState {
    /// All three events (`mousedown`, `mouseup`, `click`) fully dispatched.
    /// Set regardless of whether any of them called `preventDefault()` —
    /// prevented-default is a default-action outcome, not a postcondition
    /// failure; see `mousedown_default_prevented`/etc. on [`ClickOutcome`]
    /// for that separately.
    Verified,
    /// A listener freed (or otherwise made unresolvable) the target
    /// partway through the sequence; the human-readable reason names which
    /// step was skipped. Genuine and actionable, not swallowed.
    Failed(String),
}

/// Result of one [`perform_click`] call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickOutcome {
    pub mousedown_default_prevented: bool,
    pub mouseup_default_prevented: bool,
    pub click_default_prevented: bool,
    /// The element that implicit focus-on-click moved to, if any (`None` if
    /// `mousedown` called `preventDefault()`, if the target/nearest
    /// focusable ancestor was already active, or if nothing in the
    /// ancestor chain qualifies under [`crate::focus::is_focusable_by_default`]).
    pub focused: Option<NodeHandle>,
    pub postcondition: PostconditionState,
}

impl ClickOutcome {
    fn pending() -> Self {
        Self {
            mousedown_default_prevented: false,
            mouseup_default_prevented: false,
            click_default_prevented: false,
            focused: None,
            postcondition: PostconditionState::Verified,
        }
    }
}

fn mouse_init(kind: EventKind) -> EventInit {
    EventInit::for_kind(kind).with_detail(EventDetail::Mouse(MouseEventInit::synthetic_click()))
}

/// Synthesizes and dispatches `mousedown` -> (implicit focus) -> `mouseup`
/// -> `click` at `target`.
///
/// **Interactability precheck, deliberately narrow:** `target` must be
/// attached (its ancestor chain must reach the document root) or this
/// returns [`EventError::NotInteractable`]. This is the *only* rule
/// enforced — visibility, size, occlusion, `disabled`, and `pointer-events`
/// are explicit, called-out gaps needing M3-T12 layout/style information
/// this crate does not have. This is *stricter* than the generic
/// `add_event_listener`/`dispatch_event` primitives (see
/// [`crate::dispatch::is_attached`]'s doc comment): detached-but-resolvable
/// nodes are valid `EventTarget`s in general but cannot be meaningfully
/// clicked by a user, which is a business rule specific to this action, not
/// a change to `EventTarget` semantics.
///
/// If a listener frees or detaches `target` partway through the sequence,
/// the remaining steps are skipped and `postcondition` reports
/// [`PostconditionState::Failed`] — this is a genuine, reported outcome,
/// never a panic.
pub fn perform_click(
    document: &mut machina_dom::Document,
    registry: &mut EventTargetRegistry,
    target: NodeHandle,
) -> Result<ClickOutcome, EventError> {
    document
        .node(target)
        .map_err(|_| EventError::TargetNotFound)?;
    if !is_attached(document, target)? {
        return Err(EventError::NotInteractable);
    }

    let mut outcome = ClickOutcome::pending();

    let mousedown = dispatch_event(
        document,
        registry,
        target,
        EventKind::MouseDown,
        mouse_init(EventKind::MouseDown),
    )?;
    outcome.mousedown_default_prevented = mousedown.default_prevented;

    if !mousedown.default_prevented && document.node(target).is_ok() {
        if let Some(focus_target) = nearest_focusable(document, target)? {
            if registry.active_element() != Some(focus_target) {
                focus(document, registry, focus_target)?;
                outcome.focused = Some(focus_target);
            }
        }
    }

    if document.node(target).is_err() {
        outcome.postcondition = PostconditionState::Failed("target freed before mouseup".into());
        return Ok(outcome);
    }
    let mouseup = dispatch_event(
        document,
        registry,
        target,
        EventKind::MouseUp,
        mouse_init(EventKind::MouseUp),
    )?;
    outcome.mouseup_default_prevented = mouseup.default_prevented;

    if document.node(target).is_err() {
        outcome.postcondition = PostconditionState::Failed("target freed before click".into());
        return Ok(outcome);
    }
    let click = dispatch_event(
        document,
        registry,
        target,
        EventKind::Click,
        mouse_init(EventKind::Click),
    )?;
    outcome.click_default_prevented = click.default_prevented;

    outcome.postcondition = PostconditionState::Verified;
    Ok(outcome)
}
