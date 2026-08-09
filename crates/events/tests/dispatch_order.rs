//! Capture/target/bubble order, cancellation semantics, listener-mutation
//! snapshot correctness, non-bubbling skip, and `composed_path` snapshot
//! behavior.

mod support;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use machina_dom::{Document, NodeHandle};
use machina_events::{
    dispatch_event, CallbackIdentity, DispatchContext, Event, EventInit, EventKind,
    EventTargetRegistry,
};

use support::{append, make_element, recorder, FnListener};

struct Tree {
    doc: Document,
    registry: EventTargetRegistry,
    grandparent: NodeHandle,
    parent: NodeHandle,
    target: NodeHandle,
}

fn build_tree() -> Tree {
    let mut doc = Document::new();
    let registry = EventTargetRegistry::for_document(&doc);
    let grandparent = make_element(&mut doc, "div");
    let parent = make_element(&mut doc, "div");
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, grandparent);
    append(&mut doc, grandparent, parent);
    append(&mut doc, parent, target);
    Tree {
        doc,
        registry,
        grandparent,
        parent,
        target,
    }
}

fn add(
    registry: &mut EventTargetRegistry,
    target: NodeHandle,
    kind: EventKind,
    capture: bool,
    identity: u64,
    listener: Rc<dyn machina_events::EventListener>,
) {
    registry
        .add_event_listener(
            target,
            kind,
            capture,
            false,
            false,
            CallbackIdentity(identity),
            listener,
        )
        .expect("add_event_listener");
}

#[test]
fn capture_target_bubble_order_and_registration_order_at_target() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    add(
        &mut tree.registry,
        tree.grandparent,
        EventKind::Click,
        true,
        1,
        recorder(log.clone(), "grandparent-capture"),
    );
    add(
        &mut tree.registry,
        tree.parent,
        EventKind::Click,
        true,
        2,
        recorder(log.clone(), "parent-capture"),
    );
    // Registered bubble-flag first, capture-flag second, to prove at-target
    // order is registration order, not capture-priority order.
    add(
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        false,
        3,
        recorder(log.clone(), "target-bubble-flag"),
    );
    add(
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        true,
        4,
        recorder(log.clone(), "target-capture-flag"),
    );
    add(
        &mut tree.registry,
        tree.parent,
        EventKind::Click,
        false,
        5,
        recorder(log.clone(), "parent-bubble"),
    );
    add(
        &mut tree.registry,
        tree.grandparent,
        EventKind::Click,
        false,
        6,
        recorder(log.clone(), "grandparent-bubble"),
    );

    let outcome = dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert!(outcome.fully_completed);
    assert_eq!(
        *log.borrow(),
        vec![
            "grandparent-capture",
            "parent-capture",
            "target-bubble-flag",
            "target-capture-flag",
            "parent-bubble",
            "grandparent-bubble",
        ]
    );
}

#[test]
fn stop_propagation_finishes_current_node_but_stops_further_nodes() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    let stopper_log = log.clone();
    registry_add_stopper(&mut tree.registry, tree.parent, stopper_log);
    add(
        &mut tree.registry,
        tree.parent,
        EventKind::Click,
        false,
        2,
        recorder(log.clone(), "parent-b"),
    );
    add(
        &mut tree.registry,
        tree.grandparent,
        EventKind::Click,
        false,
        3,
        recorder(log.clone(), "grandparent-should-not-fire"),
    );

    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert_eq!(*log.borrow(), vec!["parent-a-stops", "parent-b"]);
}

fn registry_add_stopper(
    registry: &mut EventTargetRegistry,
    target: NodeHandle,
    log: Rc<RefCell<Vec<String>>>,
) {
    let listener: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            log.borrow_mut().push("parent-a-stops".to_string());
            event.stop_propagation();
        },
    ));
    registry
        .add_event_listener(
            target,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            listener,
        )
        .expect("add");
}

#[test]
fn stop_immediate_propagation_stops_remaining_listeners_on_same_node_and_further_nodes() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    let stopper_log = log.clone();
    let stopper: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            stopper_log
                .borrow_mut()
                .push("grandparent-a-stops-immediate".to_string());
            event.stop_immediate_propagation();
        },
    ));
    tree.registry
        .add_event_listener(
            tree.grandparent,
            EventKind::Click,
            true,
            false,
            false,
            CallbackIdentity(1),
            stopper,
        )
        .expect("add");
    add(
        &mut tree.registry,
        tree.grandparent,
        EventKind::Click,
        true,
        2,
        recorder(log.clone(), "grandparent-b-should-not-fire"),
    );
    add(
        &mut tree.registry,
        tree.parent,
        EventKind::Click,
        true,
        3,
        recorder(log.clone(), "parent-capture-should-not-fire"),
    );
    add(
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        true,
        4,
        recorder(log.clone(), "target-should-not-fire"),
    );

    let outcome = dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert_eq!(*log.borrow(), vec!["grandparent-a-stops-immediate"]);
    assert!(outcome.fully_completed);
}

#[test]
fn prevent_default_sets_flag_only_when_cancelable_and_not_passive() {
    let mut tree = build_tree();

    // Cancelable, active listener: sets the flag.
    let active: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            event.prevent_default();
        },
    ));
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            active,
        )
        .expect("add");
    let outcome = dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");
    assert!(outcome.default_prevented);
}

#[test]
fn prevent_default_is_a_noop_for_a_passive_listener() {
    let mut tree = build_tree();
    let observed_during_call = Rc::new(Cell::new(true));
    let observed_clone = observed_during_call.clone();

    let passive: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            event.prevent_default();
            observed_clone.set(event.default_prevented());
        },
    ));
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Click,
            false,
            false,
            true,
            CallbackIdentity(1),
            passive,
        )
        .expect("add");

    let outcome = dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert!(!outcome.default_prevented);
    assert!(!observed_during_call.get());
}

#[test]
fn prevent_default_is_a_noop_for_a_non_cancelable_event() {
    let mut tree = build_tree();
    let active: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            event.prevent_default();
        },
    ));
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Focus,
            false,
            false,
            false,
            CallbackIdentity(1),
            active,
        )
        .expect("add");

    let outcome = dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Focus,
        EventInit::for_kind(EventKind::Focus),
    )
    .expect("dispatch");

    assert!(!outcome.default_prevented);
}

#[test]
fn once_listener_fires_exactly_once_across_two_dispatches() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));
    let listener = recorder(log.clone(), "once-fired");
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Click,
            false,
            true,
            false,
            CallbackIdentity(1),
            listener,
        )
        .expect("add");

    assert_eq!(tree.registry.listener_count(tree.target), 1);
    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch 1");
    assert_eq!(tree.registry.listener_count(tree.target), 0);
    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch 2");

    assert_eq!(*log.borrow(), vec!["once-fired"]);
}

#[test]
fn listener_added_to_its_own_node_during_dispatch_does_not_fire_this_pass() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    let target = tree.target;
    let inner_log = log.clone();
    let adder: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |_event: &mut Event, ctx: &mut DispatchContext<'_>| {
            inner_log.borrow_mut().push("adder-fired".to_string());
            let new_listener = recorder(inner_log.clone(), "late-added-should-not-fire-this-pass");
            ctx.registry
                .add_event_listener(
                    target,
                    EventKind::Click,
                    false,
                    false,
                    false,
                    CallbackIdentity(99),
                    new_listener,
                )
                .expect("add during dispatch");
        },
    ));
    // Registered on target itself: the "own node" case.
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            adder,
        )
        .expect("add");

    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch 1");
    assert_eq!(*log.borrow(), vec!["adder-fired"]);
    assert_eq!(
        tree.registry.listener_count(tree.target),
        2,
        "the late-added listener is registered, just not fired this pass"
    );

    // A second, separate dispatch on the same target does pick up the newly
    // added listener (it was registered normally, just too late for the
    // in-flight snapshot taken during dispatch 1).
    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch 2");
    assert_eq!(
        *log.borrow(),
        vec![
            "adder-fired",
            "adder-fired",
            "late-added-should-not-fire-this-pass"
        ]
    );
}

#[test]
fn removed_listener_on_a_not_yet_visited_ancestor_does_not_fire() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    add(
        &mut tree.registry,
        tree.parent,
        EventKind::Click,
        true,
        7,
        recorder(log.clone(), "parent-should-be-removed"),
    );
    let parent = tree.parent;
    let remover: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |_event: &mut Event, ctx: &mut DispatchContext<'_>| {
            ctx.registry
                .remove_event_listener(parent, EventKind::Click, true, CallbackIdentity(7))
                .expect("remove");
        },
    ));
    tree.registry
        .add_event_listener(
            tree.grandparent,
            EventKind::Click,
            true,
            false,
            false,
            CallbackIdentity(1),
            remover,
        )
        .expect("add");

    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert!(log.borrow().is_empty());
}

#[test]
fn removing_a_not_yet_invoked_listener_on_the_same_node_still_fires_it() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    let target = tree.target;
    let inner_log = log.clone();
    let remover: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |_event: &mut Event, ctx: &mut DispatchContext<'_>| {
            inner_log.borrow_mut().push("remover-fired".to_string());
            ctx.registry
                .remove_event_listener(target, EventKind::Click, false, CallbackIdentity(2))
                .expect("remove");
        },
    ));
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            remover,
        )
        .expect("add");
    add(
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        false,
        2,
        recorder(log.clone(), "already-snapshotted-still-fires"),
    );

    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert_eq!(
        *log.borrow(),
        vec!["remover-fired", "already-snapshotted-still-fires"]
    );
    assert_eq!(tree.registry.listener_count(tree.target), 1);
}

#[test]
fn non_bubbling_event_skips_bubble_phase_but_still_runs_capture_and_target() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    add(
        &mut tree.registry,
        tree.parent,
        EventKind::Focus,
        true,
        1,
        recorder(log.clone(), "parent-capture"),
    );
    add(
        &mut tree.registry,
        tree.target,
        EventKind::Focus,
        false,
        2,
        recorder(log.clone(), "target-at-target"),
    );
    add(
        &mut tree.registry,
        tree.parent,
        EventKind::Focus,
        false,
        3,
        recorder(log.clone(), "parent-bubble-should-not-fire"),
    );

    let init = EventInit::for_kind(EventKind::Focus);
    assert!(!init.bubbles);
    dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Focus,
        init,
    )
    .expect("dispatch");

    assert_eq!(*log.borrow(), vec!["parent-capture", "target-at-target"]);
}

#[test]
fn composed_path_is_a_dispatch_start_snapshot_unaffected_by_mid_dispatch_mutation() {
    let mut tree = build_tree();
    let expected_before: Vec<NodeHandle> =
        vec![tree.target, tree.parent, tree.grandparent, tree.doc.root()];

    let paths: Rc<RefCell<Vec<Vec<NodeHandle>>>> = Rc::new(RefCell::new(Vec::new()));
    let parent = tree.parent;
    let paths_clone = paths.clone();
    let detacher: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |event: &mut Event, ctx: &mut DispatchContext<'_>| {
            paths_clone.borrow_mut().push(event.composed_path());
            ctx.document
                .remove_child(parent, event.target())
                .expect("detach mid-dispatch");
        },
    ));
    let paths_clone2 = paths.clone();
    let after: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            paths_clone2.borrow_mut().push(event.composed_path());
        },
    ));
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            detacher,
        )
        .expect("add");
    tree.registry
        .add_event_listener(
            tree.target,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(2),
            after,
        )
        .expect("add");

    let outcome = dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert!(outcome.fully_completed);
    let recorded = paths.borrow();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0], expected_before);
    assert_eq!(recorded[1], expected_before);
}

#[test]
fn stale_ancestor_mid_dispatch_is_skipped_gracefully_not_a_panic() {
    let mut tree = build_tree();
    let log = Rc::new(RefCell::new(Vec::new()));

    let parent = tree.parent;
    let target = tree.target;
    // Fully detach + free `parent` from within grandparent's capture phase,
    // before `parent`'s own capture turn.
    let grandparent = tree.grandparent;
    let inner_log = log.clone();
    let full_destroyer: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |_event: &mut Event, ctx: &mut DispatchContext<'_>| {
            inner_log
                .borrow_mut()
                .push("full-destroyer-fired".to_string());
            ctx.document
                .remove_child(parent, target)
                .expect("detach target from parent");
            ctx.document
                .remove_child(grandparent, parent)
                .expect("detach parent from grandparent");
            ctx.document
                .destroy_node(parent)
                .expect("destroy detached, childless parent");
        },
    ));
    tree.registry
        .add_event_listener(
            tree.grandparent,
            EventKind::Click,
            true,
            false,
            false,
            CallbackIdentity(1),
            full_destroyer,
        )
        .expect("add");
    add(
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        false,
        2,
        recorder(log.clone(), "target-still-fires"),
    );

    let outcome = dispatch_event(
        &mut tree.doc,
        &mut tree.registry,
        tree.target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert!(
        !outcome.fully_completed,
        "parent was freed mid-dispatch, so a node was skipped"
    );
    assert_eq!(
        *log.borrow(),
        vec!["full-destroyer-fired", "target-still-fires"]
    );
}
