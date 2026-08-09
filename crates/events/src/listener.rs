//! The listener trait callbacks are invoked through, plus the identity/id
//! types the registry uses to store and (idempotently) de-duplicate them.

use std::rc::Rc;

use machina_dom::Document;

use crate::event::Event;
use crate::target::EventTargetRegistry;

/// Caller-assigned bookkeeping handle for one registered listener, returned
/// by [`crate::target::EventTargetRegistry::add_event_listener`]. Opaque;
/// only meaningful as an argument back into this crate (for example
/// [`crate::target::EventTargetRegistry::remove_event_listener_by_id`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ListenerId(pub(crate) u64);

/// An opaque comparison key the caller supplies to identify "the same
/// callback" across repeated `add`/`remove` calls, mirroring
/// `EventTarget.addEventListener`'s `(type, callback, capture)` triple
/// without this crate ever needing to know what "a callback" is (a real
/// binding layer would use a JS function's persistent-handle id here).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CallbackIdentity(pub u64);

/// Mutable access a listener needs to react meaningfully to the event it
/// just received: enough to mutate the DOM (for example, a handler that
/// detaches or frees its own target) and enough to mutate this crate's
/// listener/focus registry (for example, a handler that removes another
/// listener on the same node). Deliberately just two reborrowed references,
/// not a new abstraction — `dispatch_event` itself only ever holds these two
/// mutable references, so a listener gets exactly the same capability the
/// dispatch loop has.
pub struct DispatchContext<'a> {
    pub document: &'a mut Document,
    pub registry: &'a mut EventTargetRegistry,
}

/// The result of one listener invocation. `Threw` is a forward-compatibility
/// hook for a future JS-backed listener (an exception inside a real
/// `EventListener` callback still lets dispatch continue, per spec); no
/// caller in this crate produces it today since there is no JS runtime yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListenerResult {
    Completed,
    Threw,
}

/// A registered event callback.
///
/// Deliberately **not** `Send`/`Sync`-bound (a documented deviation from the
/// original design note, which specified `EventListener: Send`): this
/// crate's listener storage snapshots callbacks as cheap `Rc` clones (see
/// [`crate::target::EventTargetRegistry`]'s module docs) so dispatch can
/// safely hand a listener mutable registry/document access without holding
/// a live borrow of the table across the call. `Rc` is never `Send`
/// regardless of `T`, so requiring `Send` here would only have forced every
/// listener (including the ergonomic `Cell`/`RefCell`-based recorders this
/// crate's own tests use) to also avoid interior mutability for no benefit,
/// since `machina_dom::Document` itself is already documented as
/// single-thread-at-a-time. A future binding layer that truly needs to move
/// a registry across threads can wrap it at that boundary.
pub trait EventListener {
    fn handle_event(&self, event: &mut Event, ctx: &mut DispatchContext<'_>) -> ListenerResult;
}

pub(crate) struct ListenerEntry {
    pub(crate) id: ListenerId,
    pub(crate) event_kind: crate::event::EventKind,
    pub(crate) capture: bool,
    pub(crate) once: bool,
    pub(crate) passive: bool,
    pub(crate) identity: CallbackIdentity,
    pub(crate) callback: Rc<dyn EventListener>,
}

impl Clone for ListenerEntry {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            event_kind: self.event_kind,
            capture: self.capture,
            once: self.once,
            passive: self.passive,
            identity: self.identity,
            callback: Rc::clone(&self.callback),
        }
    }
}
