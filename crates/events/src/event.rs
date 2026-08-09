//! The event object dispatched through a node's listener list: kind, phase,
//! the dispatch-start path snapshot, and the standard
//! prevent-default/stop-propagation controls.

use machina_dom::NodeHandle;

use crate::keyboard::KeyboardEventInit;
use crate::mouse::MouseEventInit;

/// The kind of synthetic event. Left open (non-exhaustive) since new
/// variants (for example a future `interaction.type.v1`'s `input`/`change`)
/// are expected to be added without restructuring dispatch — mirrors
/// `machina_dom::NodeKind`'s own "open enum" convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum EventKind {
    MouseDown,
    MouseUp,
    Click,
    Focus,
    Blur,
    FocusIn,
    FocusOut,
    KeyDown,
    KeyUp,
}

impl EventKind {
    /// Spec-typical `bubbles`/`cancelable` defaults for this kind, used by
    /// [`EventInit::for_kind`]. `focus`/`blur` do not bubble (only their
    /// `focusin`/`focusout` counterparts do) and are not cancelable, per the
    /// DOM Focus Event spec.
    pub fn default_flags(self) -> (bool, bool) {
        match self {
            Self::MouseDown | Self::MouseUp | Self::Click => (true, true),
            Self::Focus | Self::Blur => (false, false),
            Self::FocusIn | Self::FocusOut => (true, false),
            Self::KeyDown | Self::KeyUp => (true, true),
        }
    }
}

/// Which propagation phase a listener is currently being invoked in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPhase {
    None,
    Capturing,
    AtTarget,
    Bubbling,
}

/// Kind-specific payload. `None` for focus events, which carry no extra
/// data beyond the base `Event` fields.
#[derive(Clone, Debug, PartialEq)]
pub enum EventDetail {
    None,
    Mouse(MouseEventInit),
    Keyboard(KeyboardEventInit),
}

/// Construction parameters for [`crate::dispatch::dispatch_event`]. Use
/// [`EventInit::for_kind`] for spec-typical `bubbles`/`cancelable` defaults,
/// then override fields (for example `detail`) as needed.
#[derive(Clone, Debug, PartialEq)]
pub struct EventInit {
    pub bubbles: bool,
    pub cancelable: bool,
    /// Always `false` today: no `ShadowRoot`/`NodeKind` exists yet (see
    /// `machina_dom`'s M2-T05 non-goals). Present so M3-T04's shadow-DOM
    /// work only has to change `composed_path` construction, not this
    /// struct's shape.
    pub composed: bool,
    pub is_trusted: bool,
    pub detail: EventDetail,
}

impl EventInit {
    pub fn for_kind(kind: EventKind) -> Self {
        let (bubbles, cancelable) = kind.default_flags();
        Self {
            bubbles,
            cancelable,
            composed: false,
            is_trusted: true,
            detail: EventDetail::None,
        }
    }

    pub fn with_detail(mut self, detail: EventDetail) -> Self {
        self.detail = detail;
        self
    }
}

/// One in-flight (or already-completed) event. Mutated in place by
/// listeners via `prevent_default`/`stop_propagation`/
/// `stop_immediate_propagation` as it is threaded through
/// [`crate::dispatch::dispatch_event`]'s capture/target/bubble loop.
#[derive(Clone, Debug)]
pub struct Event {
    kind: EventKind,
    bubbles: bool,
    cancelable: bool,
    composed: bool,
    is_trusted: bool,
    target: NodeHandle,
    current_target: Option<NodeHandle>,
    phase: EventPhase,
    /// Dispatch-start snapshot, root-first (`path[0]` is the outermost
    /// ancestor, `path[last]` is `target`). Never rebuilt mid-dispatch, even
    /// if a listener mutates the tree — see [`Event::composed_path`].
    path: Vec<NodeHandle>,
    default_prevented: bool,
    propagation_stopped: bool,
    immediate_propagation_stopped: bool,
    in_passive_listener: bool,
    detail: EventDetail,
}

impl Event {
    pub(crate) fn new(
        kind: EventKind,
        init: &EventInit,
        target: NodeHandle,
        path: Vec<NodeHandle>,
    ) -> Self {
        Self {
            kind,
            bubbles: init.bubbles,
            cancelable: init.cancelable,
            composed: init.composed,
            is_trusted: init.is_trusted,
            target,
            current_target: None,
            phase: EventPhase::None,
            path,
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            in_passive_listener: false,
            detail: init.detail.clone(),
        }
    }

    pub(crate) fn set_phase(&mut self, phase: EventPhase) {
        self.phase = phase;
    }

    pub(crate) fn set_current_target(&mut self, handle: Option<NodeHandle>) {
        self.current_target = handle;
    }

    pub(crate) fn set_in_passive_listener(&mut self, value: bool) {
        self.in_passive_listener = value;
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub fn bubbles(&self) -> bool {
        self.bubbles
    }

    pub fn cancelable(&self) -> bool {
        self.cancelable
    }

    pub fn composed(&self) -> bool {
        self.composed
    }

    pub fn is_trusted(&self) -> bool {
        self.is_trusted
    }

    pub fn target(&self) -> NodeHandle {
        self.target
    }

    pub fn current_target(&self) -> Option<NodeHandle> {
        self.current_target
    }

    pub fn phase(&self) -> EventPhase {
        self.phase
    }

    pub fn default_prevented(&self) -> bool {
        self.default_prevented
    }

    pub fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    pub fn immediate_propagation_stopped(&self) -> bool {
        self.immediate_propagation_stopped
    }

    pub fn in_passive_listener(&self) -> bool {
        self.in_passive_listener
    }

    pub fn detail(&self) -> &EventDetail {
        &self.detail
    }

    pub fn mouse_detail(&self) -> Option<&MouseEventInit> {
        match &self.detail {
            EventDetail::Mouse(mouse) => Some(mouse),
            _ => None,
        }
    }

    pub fn keyboard_detail(&self) -> Option<&KeyboardEventInit> {
        match &self.detail {
            EventDetail::Keyboard(keyboard) => Some(keyboard),
            _ => None,
        }
    }

    /// The dispatch-start ancestor chain, target-first (index `0`) through
    /// the outermost ancestor (last index) — the DOM `composedPath()`
    /// ordering. A dispatch-start snapshot: mutating the tree from inside a
    /// listener never changes what this returns for the remainder of this
    /// dispatch.
    pub fn composed_path(&self) -> Vec<NodeHandle> {
        self.path.iter().rev().copied().collect()
    }

    /// No-ops (does not set `default_prevented`) when the event is not
    /// `cancelable`, or while a `passive` listener is currently running —
    /// matching the DOM spec's passive-listener suppression instead of
    /// throwing.
    pub fn prevent_default(&mut self) {
        if !self.cancelable || self.in_passive_listener {
            return;
        }
        self.default_prevented = true;
    }

    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Also implies [`Event::stop_propagation`], per spec.
    pub fn stop_immediate_propagation(&mut self) {
        self.propagation_stopped = true;
        self.immediate_propagation_stopped = true;
    }
}
