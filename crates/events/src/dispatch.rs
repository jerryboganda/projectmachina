//! The capture/target/bubble dispatch algorithm.

use machina_dom::{Document, NodeHandle, MAX_ANCESTOR_WALK};

use crate::error::EventError;
use crate::event::{Event, EventInit, EventKind, EventPhase};
pub use crate::listener::DispatchContext;
use crate::target::EventTargetRegistry;

/// The outcome of one [`dispatch_event`] call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchOutcome {
    pub default_prevented: bool,
    /// `false` only when an ancestor (or the target, if a capture-phase
    /// listener detached/freed it before the at-target phase ran) stopped
    /// resolving partway through this dispatch and its listeners were
    /// skipped as a result. A listener calling `stop_propagation()` is a
    /// normal, intentional early exit and does **not** set this to `false`.
    pub fully_completed: bool,
}

/// Root-first ancestor chain from (but not including) `target` up to and
/// including the document root, or an empty vec if `target` has no parent.
/// Bounded by `machina_dom::MAX_ANCESTOR_WALK`, matching the guard style
/// `machina_dom`'s own ancestor walks use.
pub(crate) fn ancestors_root_first(
    document: &Document,
    target: NodeHandle,
) -> Result<Vec<NodeHandle>, EventError> {
    let mut chain = Vec::new();
    let mut current = document
        .node(target)
        .map_err(|_| EventError::TargetNotFound)?
        .parent();
    let mut steps = 0usize;
    while let Some(handle) = current {
        steps += 1;
        if steps > MAX_ANCESTOR_WALK {
            return Err(EventError::from(machina_dom::DomError::DepthLimitExceeded));
        }
        chain.push(handle);
        current = document.node(handle)?.parent();
    }
    chain.reverse();
    Ok(chain)
}

/// `true` if `target`'s ancestor chain reaches the document root (or
/// `target` *is* the root). This is `perform_click`'s narrow
/// interactability gate, not a general `EventTarget` precondition — see
/// this crate's top-level docs.
pub fn is_attached(document: &Document, target: NodeHandle) -> Result<bool, EventError> {
    if target == document.root() {
        return Ok(true);
    }
    let chain = ancestors_root_first(document, target)?;
    Ok(chain.first() == Some(&document.root()))
}

/// Dispatches a synthetic event at `target` through the capture, at-target,
/// and (if `init.bubbles`) bubble phases.
///
/// The propagation path is a snapshot taken once, up front, from `target`'s
/// ancestor chain at the moment of this call — not a live walk. If a
/// listener mutates the tree mid-dispatch, remaining phases still iterate
/// the originally computed path; a node is simply skipped (not treated as a
/// fatal error) if it no longer resolves when its turn comes.
pub fn dispatch_event(
    document: &mut Document,
    registry: &mut EventTargetRegistry,
    target: NodeHandle,
    kind: EventKind,
    init: EventInit,
) -> Result<DispatchOutcome, EventError> {
    let ancestors = ancestors_root_first(document, target)?; // root..parent-of-target
    let mut full_path = ancestors.clone();
    full_path.push(target);

    let mut event = Event::new(kind, &init, target, full_path);
    let bubbles = init.bubbles;

    // (node, phase, capture-filter) in dispatch order.
    let mut steps: Vec<(NodeHandle, EventPhase, Option<bool>)> = ancestors
        .iter()
        .map(|handle| (*handle, EventPhase::Capturing, Some(true)))
        .collect();
    steps.push((target, EventPhase::AtTarget, None));
    if bubbles {
        steps.extend(
            ancestors
                .iter()
                .rev()
                .map(|handle| (*handle, EventPhase::Bubbling, Some(false))),
        );
    }

    let mut fully_completed = true;

    'dispatch: for (node, phase, want_capture) in steps {
        if document.node(node).is_err() {
            fully_completed = false;
            continue;
        }
        event.set_phase(phase);
        event.set_current_target(Some(node));

        let snapshot = registry.snapshot_listeners(node, kind, want_capture);
        for entry in snapshot {
            event.set_in_passive_listener(entry.passive);
            let mut ctx = DispatchContext {
                document: &mut *document,
                registry: &mut *registry,
            };
            entry.callback.handle_event(&mut event, &mut ctx);
            if entry.once {
                let _ = registry.remove_event_listener_by_id(node, entry.id);
            }
            if event.immediate_propagation_stopped() {
                break;
            }
        }

        if event.propagation_stopped() {
            break 'dispatch;
        }
    }

    Ok(DispatchOutcome {
        default_prevented: event.default_prevented(),
        fully_completed,
    })
}
