//! Shared test-only helpers: tree construction and small `EventListener`
//! implementations used across this crate's test suites.
//!
//! Each `tests/*.rs` integration test file compiles its own copy of this
//! module, and not every helper is used by every file — `#![allow(dead_code)]`
//! avoids spurious per-binary warnings for that reason, matching
//! `machina_dom`'s own shared test-support module.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use machina_dom::{Document, NodeHandle};
use machina_events::{DispatchContext, Event, EventListener, ListenerResult};

pub fn make_element(doc: &mut Document, tag: &str) -> NodeHandle {
    doc.create_element(tag)
        .expect("create element")
        .node_handle()
}

/// Appends `child` under `parent`, panicking (test-only) on failure.
pub fn append(doc: &mut Document, parent: NodeHandle, child: NodeHandle) {
    doc.append_child(parent, child).expect("append_child");
}

/// Creates one `<div>` wrapper element and appends it to `doc.root()`,
/// returning its handle so tests that need more than one top-level element
/// have somewhere to attach them — `Document`'s root only permits a single
/// `Element` child directly.
pub fn root_container(doc: &mut Document) -> NodeHandle {
    let container = make_element(doc, "div");
    let root = doc.root();
    append(doc, root, container);
    container
}

/// A listener that runs an arbitrary closure, useful for both recording
/// call order and mutating the document/registry mid-dispatch.
pub struct FnListener<F>(pub F)
where
    F: Fn(&mut Event, &mut DispatchContext<'_>);

impl<F> EventListener for FnListener<F>
where
    F: Fn(&mut Event, &mut DispatchContext<'_>),
{
    fn handle_event(&self, event: &mut Event, ctx: &mut DispatchContext<'_>) -> ListenerResult {
        (self.0)(event, ctx);
        ListenerResult::Completed
    }
}

/// Appends `label` to a shared log every time it fires; does nothing else.
pub fn recorder(log: Rc<RefCell<Vec<String>>>, label: &'static str) -> Rc<dyn EventListener> {
    Rc::new(FnListener(
        move |_event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            log.borrow_mut().push(label.to_string());
        },
    ))
}

/// A listener whose `Drop` increments a shared counter — used to prove
/// listener storage does not leak references past cleanup.
pub struct DropCountingListener(pub Rc<Cell<u32>>);

impl EventListener for DropCountingListener {
    fn handle_event(&self, _event: &mut Event, _ctx: &mut DispatchContext<'_>) -> ListenerResult {
        ListenerResult::Completed
    }
}

impl Drop for DropCountingListener {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
