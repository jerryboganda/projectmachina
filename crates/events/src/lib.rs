//! Event dispatch, the active-element/focus model, and synthetic
//! mouse/keyboard input over `machina_dom` node handles.
//!
//! Depends only on `machina_dom` (a path dependency) — no dependency on
//! `machina-event-loop` (dispatch itself is a synchronous function call, not
//! a scheduled task; deciding *when* to call it is the composing layer's
//! job), `machina-runtime-v8` (a listener callback is a generic trait
//! object, not a V8 handle), or any command-bus/session/protocol crate. No
//! `unsafe`, no `unwrap`/`expect`/`panic!` on any caller-reachable path.
//!
//! ## Two-tier attachment model
//!
//! The generic primitives ([`EventTargetRegistry::add_event_listener`],
//! [`dispatch_event`]) only distinguish "resolvable" (attached *or*
//! detached-but-live) from "stale" (freed/wrong-document/closed) — matching
//! real browsers, where `EventTarget.addEventListener`/`dispatchEvent` have
//! no "must be connected" precondition (`document.createElement` supports
//! listeners immediately, before insertion). [`perform_click`] adds its own
//! narrower, action-specific attachment gate on top of that — a business
//! rule for "can a user meaningfully click this," not a change to
//! `EventTarget` semantics. See [`dispatch::is_attached`] and
//! [`action::perform_click`]'s doc comments.
//!
//! ## Scope boundary: this crate receives resolved handles only
//!
//! Selector -> [`machina_dom::NodeHandle`] resolution (and the
//! `ELEMENT_AMBIGUOUS` case) is M2-T10's job, upstream of this crate;
//! mapping this crate's error/outcome types onto `CanonicalErrorCode` and
//! wiring `interaction.click.v1` through the command bus is native-core's
//! job, downstream of it. Neither is implemented here — see the crate's
//! evidence file (`.agent-state/evidence/M2-T11.md`) for the explicit
//! cross-task dependency this leaves.

mod action;
mod dispatch;
mod error;
mod event;
mod focus;
mod keyboard;
mod listener;
mod mouse;
mod target;

pub use action::{perform_click, ClickOutcome, PostconditionState};
pub use dispatch::{dispatch_event, is_attached, DispatchOutcome};
pub use error::EventError;
pub use event::{Event, EventDetail, EventInit, EventKind, EventPhase};
pub use focus::{blur, focus, is_focusable_by_default, nearest_focusable, FocusOutcome};
pub use keyboard::KeyboardEventInit;
pub use listener::{CallbackIdentity, DispatchContext, EventListener, ListenerId, ListenerResult};
pub use mouse::{MouseButton, MouseEventInit};
pub use target::EventTargetRegistry;
