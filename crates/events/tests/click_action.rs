//! `perform_click`: happy path, mousedown-preventDefault suppressing
//! implicit focus, click-preventDefault not affecting
//! `postcondition = Verified`, and graceful `Failed` postcondition (not a
//! panic) when a listener frees the target mid-dispatch.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use machina_dom::Document;
use machina_events::{
    perform_click, CallbackIdentity, DispatchContext, Event, EventKind, EventTargetRegistry,
    PostconditionState,
};

use support::{append, make_element, FnListener};

#[test]
fn happy_path_dispatches_all_three_events_and_focuses_the_target() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let parent = make_element(&mut doc, "div");
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, parent);
    append(&mut doc, parent, target);

    let log = Rc::new(RefCell::new(Vec::new()));
    for (kind, identity, label) in [
        (EventKind::MouseDown, 1u64, "mousedown"),
        (EventKind::MouseUp, 2u64, "mouseup"),
        (EventKind::Click, 3u64, "click"),
        (EventKind::Focus, 4u64, "focus"),
    ] {
        registry
            .add_event_listener(
                target,
                kind,
                false,
                false,
                false,
                CallbackIdentity(identity),
                support::recorder(log.clone(), label),
            )
            .expect("add");
    }

    let outcome = perform_click(&mut doc, &mut registry, target).expect("perform_click");

    assert!(!outcome.mousedown_default_prevented);
    assert!(!outcome.mouseup_default_prevented);
    assert!(!outcome.click_default_prevented);
    assert_eq!(outcome.focused, Some(target));
    assert_eq!(outcome.postcondition, PostconditionState::Verified);
    assert_eq!(registry.active_element(), Some(target));
    assert_eq!(
        *log.borrow(),
        vec!["mousedown", "focus", "mouseup", "click"]
    );
}

#[test]
fn mousedown_prevent_default_suppresses_implicit_focus() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, target);

    let preventer: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            event.prevent_default();
        },
    ));
    registry
        .add_event_listener(
            target,
            EventKind::MouseDown,
            false,
            false,
            false,
            CallbackIdentity(1),
            preventer,
        )
        .expect("add");

    let outcome = perform_click(&mut doc, &mut registry, target).expect("perform_click");

    assert!(outcome.mousedown_default_prevented);
    assert_eq!(outcome.focused, None);
    assert_eq!(registry.active_element(), None);
    assert_eq!(outcome.postcondition, PostconditionState::Verified);
}

#[test]
fn click_prevent_default_does_not_affect_postcondition_verified() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, target);

    let preventer: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        |event: &mut Event, _ctx: &mut DispatchContext<'_>| {
            event.prevent_default();
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
            preventer,
        )
        .expect("add");

    let outcome = perform_click(&mut doc, &mut registry, target).expect("perform_click");

    assert!(outcome.click_default_prevented);
    assert_eq!(outcome.postcondition, PostconditionState::Verified);
}

#[test]
fn mid_dispatch_target_freeing_produces_a_graceful_failed_postcondition_not_a_panic() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let parent = make_element(&mut doc, "div");
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, parent);
    append(&mut doc, parent, target);

    let destroyer: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |event: &mut Event, ctx: &mut DispatchContext<'_>| {
            let target = event.target();
            ctx.document
                .remove_child(parent, target)
                .expect("detach target");
            ctx.document
                .destroy_node(target)
                .expect("destroy detached, childless target");
        },
    ));
    registry
        .add_event_listener(
            target,
            EventKind::MouseDown,
            false,
            false,
            false,
            CallbackIdentity(1),
            destroyer,
        )
        .expect("add");

    let outcome =
        perform_click(&mut doc, &mut registry, target).expect("perform_click does not error out");

    assert!(!outcome.mousedown_default_prevented);
    assert_eq!(
        outcome.focused, None,
        "target was freed before the focus step could run"
    );
    match outcome.postcondition {
        PostconditionState::Failed(reason) => {
            assert!(reason.contains("mouseup"), "unexpected reason: {reason}")
        }
        PostconditionState::Verified => panic!("expected Failed postcondition, got Verified"),
    }
}

#[test]
fn click_on_a_non_focusable_target_leaves_focus_unset() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "div"); // not in the built-in focusable tag set
    let root = doc.root();
    append(&mut doc, root, target);

    let outcome = perform_click(&mut doc, &mut registry, target).expect("perform_click");

    assert_eq!(outcome.focused, None);
    assert_eq!(registry.active_element(), None);
    assert_eq!(outcome.postcondition, PostconditionState::Verified);
}

#[test]
fn click_on_an_already_focused_target_does_not_refocus() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, target);

    let log = Rc::new(RefCell::new(Vec::new()));
    registry
        .add_event_listener(
            target,
            EventKind::Focus,
            false,
            false,
            false,
            CallbackIdentity(1),
            support::recorder(log.clone(), "focus"),
        )
        .expect("add");

    machina_events::focus(&mut doc, &mut registry, target).expect("manual focus");
    assert_eq!(*log.borrow(), vec!["focus"]);

    let outcome = perform_click(&mut doc, &mut registry, target).expect("perform_click");

    assert_eq!(
        outcome.focused, None,
        "already active; perform_click does not report a fresh focus change"
    );
    assert_eq!(*log.borrow(), vec!["focus"], "no second focus event fired");
}
