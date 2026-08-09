//! The two-tier attachment model (§6 of the design): generic
//! `add_event_listener`/`dispatch_event` work on never-attached and
//! detached nodes; `perform_click` rejects both, on the exact same handles
//! that dispatch happily accepts.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use machina_dom::Document;
use machina_events::{
    dispatch_event, is_attached, perform_click, CallbackIdentity, EventError, EventInit, EventKind,
    EventTargetRegistry,
};

use support::{append, make_element, recorder};

#[test]
fn listener_and_dispatch_work_on_a_never_attached_node() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let orphan = make_element(&mut doc, "div"); // never appended anywhere

    assert!(!is_attached(&doc, orphan).expect("check"));

    let log = Rc::new(RefCell::new(Vec::new()));
    registry
        .add_event_listener(
            orphan,
            EventKind::Click,
            false,
            false,
            false,
            CallbackIdentity(1),
            recorder(log.clone(), "fired"),
        )
        .expect("add on orphan");

    let outcome = dispatch_event(
        &mut doc,
        &mut registry,
        orphan,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch on orphan");

    assert!(outcome.fully_completed);
    assert_eq!(*log.borrow(), vec!["fired"]);
}

#[test]
fn dispatch_works_on_a_node_detached_after_being_attached() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "div");
    let root = doc.root();
    append(&mut doc, root, target);
    doc.remove_child(doc.root(), target).expect("detach");

    assert!(!is_attached(&doc, target).expect("check"));

    let log = Rc::new(RefCell::new(Vec::new()));
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
    dispatch_event(
        &mut doc,
        &mut registry,
        target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    )
    .expect("dispatch");

    assert_eq!(*log.borrow(), vec!["fired"]);
}

#[test]
fn dispatch_on_a_freed_handle_fails_with_a_typed_error_not_a_panic() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "div");
    let root = doc.root();
    append(&mut doc, root, target);
    doc.remove_child(doc.root(), target).expect("detach");
    doc.destroy_node(target).expect("destroy");

    let result = dispatch_event(
        &mut doc,
        &mut registry,
        target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    );
    assert_eq!(result, Err(EventError::TargetNotFound));
}

#[test]
fn add_event_listener_rejects_a_cross_document_handle() {
    let mut doc_a = Document::new();
    let mut doc_b = Document::new();
    let mut registry_a = EventTargetRegistry::for_document(&doc_a);
    let element_in_b = make_element(&mut doc_b, "div");
    let root = doc_b.root();
    append(&mut doc_b, root, element_in_b);

    let result = registry_a.add_event_listener(
        element_in_b,
        EventKind::Click,
        false,
        false,
        false,
        CallbackIdentity(1),
        recorder(Rc::new(RefCell::new(Vec::new())), "unused"),
    );
    assert_eq!(result, Err(EventError::WrongDocument));

    // `dispatch_event` collapses cross-document into the same
    // "does not resolve here" outcome as any other unresolvable handle.
    let dispatch_result = dispatch_event(
        &mut doc_a,
        &mut registry_a,
        element_in_b,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    );
    assert_eq!(dispatch_result, Err(EventError::TargetNotFound));
}

#[test]
fn perform_click_rejects_a_never_attached_node() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let orphan = make_element(&mut doc, "button");

    let result = perform_click(&mut doc, &mut registry, orphan);
    assert_eq!(result, Err(EventError::NotInteractable));
}

#[test]
fn perform_click_succeeds_on_an_attached_node_that_generic_dispatch_also_accepts() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, target);

    let outcome = perform_click(&mut doc, &mut registry, target).expect("perform_click");
    assert_eq!(
        outcome.postcondition,
        machina_events::PostconditionState::Verified
    );
}

#[test]
fn perform_click_rejects_the_exact_same_handle_generic_dispatch_accepts_once_detached() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, target);
    doc.remove_child(doc.root(), target).expect("detach");

    // Contrast, on the exact same handle: the generic primitive still
    // works (§6), but the action-specific interactability gate rejects it.
    let dispatch_outcome = dispatch_event(
        &mut doc,
        &mut registry,
        target,
        EventKind::Click,
        EventInit::for_kind(EventKind::Click),
    );
    assert!(
        dispatch_outcome.is_ok(),
        "generic dispatch still accepts a detached-but-resolvable target"
    );

    let click_result = perform_click(&mut doc, &mut registry, target);
    assert_eq!(click_result, Err(EventError::NotInteractable));
}
