//! No leaked listener references, a single `handle_document_teardown` call
//! (not a per-node loop), and re-attachment preserving listeners.

mod support;

use std::cell::Cell;
use std::rc::Rc;

use machina_dom::Document;
use machina_events::{dispatch_event, CallbackIdentity, EventInit, EventKind, EventTargetRegistry};

use support::{append, make_element, recorder, root_container, DropCountingListener};

#[test]
fn freeing_a_node_drops_its_listeners_with_no_leak() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let parent = make_element(&mut doc, "div");
    let a = make_element(&mut doc, "div");
    let root = doc.root();
    append(&mut doc, root, parent);
    append(&mut doc, parent, a);
    doc.remove_child(parent, a).expect("detach a");

    let counter = Rc::new(Cell::new(0u32));
    let listener: Rc<dyn machina_events::EventListener> =
        Rc::new(DropCountingListener(counter.clone()));
    registry
        .add_event_listener(
            a,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            listener,
        )
        .expect("add");
    assert_eq!(counter.get(), 0);

    doc.destroy_node(a).expect("destroy detached childless a");
    registry.handle_node_freed(a);

    assert_eq!(
        counter.get(),
        1,
        "the registry's Rc clone was the last reference"
    );
    assert_eq!(registry.listener_count(a), 0);
}

#[test]
fn document_teardown_drops_every_node_listener_in_one_call_not_a_per_node_loop() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);

    let container = root_container(&mut doc);
    let counter = Rc::new(Cell::new(0u32));
    let mut nodes = Vec::new();
    for i in 0..5u64 {
        let node = make_element(&mut doc, "div");
        append(&mut doc, container, node);
        let listener: Rc<dyn machina_events::EventListener> =
            Rc::new(DropCountingListener(counter.clone()));
        registry
            .add_event_listener(
                node,
                EventKind::Click,
                false,
                false,
                false,
                CallbackIdentity(i),
                listener,
            )
            .expect("add");
        nodes.push(node);
    }
    assert_eq!(counter.get(), 0);

    let document_id = doc.id();
    registry.handle_document_teardown(document_id);

    assert_eq!(
        counter.get(),
        5,
        "a single teardown call released every node's listeners"
    );
    for node in nodes {
        assert_eq!(registry.listener_count(node), 0);
    }
}

#[test]
fn detaching_and_reattaching_a_node_preserves_its_listeners() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let parent_a = make_element(&mut doc, "div");
    let parent_b = make_element(&mut doc, "div");
    let target = make_element(&mut doc, "div");
    let container = root_container(&mut doc);
    append(&mut doc, container, parent_a);
    append(&mut doc, container, parent_b);
    append(&mut doc, parent_a, target);

    let log = Rc::new(std::cell::RefCell::new(Vec::new()));
    registry
        .add_event_listener(
            target,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            recorder(log.clone(), "fired"),
        )
        .expect("add");
    assert_eq!(registry.listener_count(target), 1);

    dispatch_event(
        &mut doc,
        &mut registry,
        target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch while attached");
    assert_eq!(*log.borrow(), vec!["fired"]);
    assert_eq!(registry.listener_count(target), 1);

    doc.remove_child(parent_a, target).expect("detach");
    assert_eq!(
        registry.listener_count(target),
        1,
        "detaching does not touch listener storage"
    );
    dispatch_event(
        &mut doc,
        &mut registry,
        target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch while detached");
    assert_eq!(*log.borrow(), vec!["fired", "fired"]);

    doc.append_child(parent_b, target)
        .expect("re-attach under a different parent");
    assert_eq!(
        registry.listener_count(target),
        1,
        "re-attachment preserves listeners"
    );
    dispatch_event(
        &mut doc,
        &mut registry,
        target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch after re-attach");
    assert_eq!(*log.borrow(), vec!["fired", "fired", "fired"]);
}
