//! Focus/blur ordering, no-op-if-already-active, minimal focusability, and
//! DOM-lifecycle interop (detach/free/teardown).

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use machina_dom::{Document, NodeChange, NodeHandle};
use machina_events::{
    blur, focus, is_focusable_by_default, nearest_focusable, CallbackIdentity, DispatchContext,
    Event, EventKind, EventTargetRegistry, FocusOutcome,
};

use support::{append, make_element, recorder, root_container, FnListener};

fn add_recorder(
    registry: &mut EventTargetRegistry,
    target: NodeHandle,
    kind: EventKind,
    identity: u64,
    log: Rc<RefCell<Vec<String>>>,
    label: &'static str,
) {
    registry
        .add_event_listener(
            target,
            kind,
            false,
            false,
            false,
            CallbackIdentity(identity),
            recorder(log, label),
        )
        .expect("add_event_listener");
}

#[test]
fn focus_on_first_target_fires_focus_then_focusin() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, target);

    let log = Rc::new(RefCell::new(Vec::new()));
    add_recorder(
        &mut registry,
        target,
        EventKind::Focus,
        1,
        log.clone(),
        "focus",
    );
    add_recorder(
        &mut registry,
        target,
        EventKind::FocusIn,
        2,
        log.clone(),
        "focusin",
    );

    let outcome = focus(&mut doc, &mut registry, target).expect("focus");
    assert_eq!(outcome, FocusOutcome::Changed { previous: None });
    assert_eq!(registry.active_element(), Some(target));
    assert_eq!(*log.borrow(), vec!["focus", "focusin"]);
}

#[test]
fn focus_is_a_noop_if_already_active() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let target = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, target);

    let log = Rc::new(RefCell::new(Vec::new()));
    add_recorder(
        &mut registry,
        target,
        EventKind::Focus,
        1,
        log.clone(),
        "focus",
    );

    focus(&mut doc, &mut registry, target).expect("focus 1");
    assert_eq!(*log.borrow(), vec!["focus"]);

    let outcome = focus(&mut doc, &mut registry, target).expect("focus 2");
    assert_eq!(outcome, FocusOutcome::NoOp);
    assert_eq!(*log.borrow(), vec!["focus"], "no additional event fired");
}

#[test]
fn focus_transition_fires_blur_focusout_at_old_then_focus_focusin_at_new_with_active_element_already_updated(
) {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let a = make_element(&mut doc, "button");
    let b = make_element(&mut doc, "button");
    let container = root_container(&mut doc);
    append(&mut doc, container, a);
    append(&mut doc, container, b);

    let log = Rc::new(RefCell::new(Vec::new()));
    let observed_active_during_blur: Rc<RefCell<Option<NodeHandle>>> = Rc::new(RefCell::new(None));
    let observed_clone = observed_active_during_blur.clone();
    let blur_listener: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |_event: &mut Event, ctx: &mut DispatchContext<'_>| {
            *observed_clone.borrow_mut() = ctx.registry.active_element();
        },
    ));
    registry
        .add_event_listener(
            a,
            EventKind::Blur,
            false,
            false,
            false,
            CallbackIdentity(1),
            blur_listener,
        )
        .expect("add");
    add_recorder(&mut registry, a, EventKind::Blur, 2, log.clone(), "blur@a");
    add_recorder(
        &mut registry,
        a,
        EventKind::FocusOut,
        3,
        log.clone(),
        "focusout@a",
    );
    add_recorder(
        &mut registry,
        b,
        EventKind::Focus,
        4,
        log.clone(),
        "focus@b",
    );
    add_recorder(
        &mut registry,
        b,
        EventKind::FocusIn,
        5,
        log.clone(),
        "focusin@b",
    );

    focus(&mut doc, &mut registry, a).expect("focus a");
    log.borrow_mut().clear();

    let outcome = focus(&mut doc, &mut registry, b).expect("focus b");
    assert_eq!(outcome, FocusOutcome::Changed { previous: Some(a) });
    assert_eq!(
        *log.borrow(),
        vec!["blur@a", "focusout@a", "focus@b", "focusin@b"]
    );
    // Deliberate ordering: active_element is set to the NEW target before
    // the OLD target's blur/focusout even fire.
    assert_eq!(*observed_active_during_blur.borrow(), Some(b));
    assert_eq!(registry.active_element(), Some(b));
}

#[test]
fn standalone_blur_fires_at_old_active_element_while_still_active_then_clears_with_no_fallback() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let a = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, a);

    let log = Rc::new(RefCell::new(Vec::new()));
    let observed_active_during_blur: Rc<RefCell<Option<NodeHandle>>> = Rc::new(RefCell::new(None));
    let observed_clone = observed_active_during_blur.clone();
    let blur_listener: Rc<dyn machina_events::EventListener> = Rc::new(FnListener(
        move |_event: &mut Event, ctx: &mut DispatchContext<'_>| {
            *observed_clone.borrow_mut() = ctx.registry.active_element();
        },
    ));
    registry
        .add_event_listener(
            a,
            EventKind::Blur,
            false,
            false,
            false,
            CallbackIdentity(1),
            blur_listener,
        )
        .expect("add");
    add_recorder(&mut registry, a, EventKind::Blur, 2, log.clone(), "blur@a");
    add_recorder(
        &mut registry,
        a,
        EventKind::FocusOut,
        3,
        log.clone(),
        "focusout@a",
    );

    focus(&mut doc, &mut registry, a).expect("focus a");
    log.borrow_mut().clear();

    let outcome = blur(&mut doc, &mut registry).expect("blur");
    assert_eq!(outcome, FocusOutcome::Changed { previous: Some(a) });
    assert_eq!(*log.borrow(), vec!["blur@a", "focusout@a"]);
    // Opposite ordering from the transition case: blur() fires while the
    // old element is *still* active, then clears afterward — there is no
    // new target to set first.
    assert_eq!(*observed_active_during_blur.borrow(), Some(a));
    assert_eq!(registry.active_element(), None);
}

#[test]
fn blur_is_a_noop_when_nothing_is_focused() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let outcome = blur(&mut doc, &mut registry).expect("blur");
    assert_eq!(outcome, FocusOutcome::NoOp);
}

#[test]
fn tabindex_and_builtin_tags_are_focusable_by_default() {
    let mut doc = Document::new();
    let div = doc.create_element("div").expect("create");
    doc.set_attribute(div, "tabindex", "0").expect("set");
    assert!(is_focusable_by_default(&doc, div.node_handle()).expect("check"));

    let negative = doc.create_element("div").expect("create");
    doc.set_attribute(negative, "tabindex", "-1").expect("set");
    assert!(is_focusable_by_default(&doc, negative.node_handle()).expect("check"));

    let bad_tabindex = doc.create_element("div").expect("create");
    doc.set_attribute(bad_tabindex, "tabindex", "not-a-number")
        .expect("set");
    assert!(!is_focusable_by_default(&doc, bad_tabindex.node_handle()).expect("check"));

    for tag in ["button", "input", "select", "textarea", "iframe"] {
        let element = doc.create_element(tag).expect("create");
        assert!(
            is_focusable_by_default(&doc, element.node_handle()).expect("check"),
            "{tag} should be focusable"
        );
    }

    let anchor_without_href = doc.create_element("a").expect("create");
    assert!(!is_focusable_by_default(&doc, anchor_without_href.node_handle()).expect("check"));
    let anchor_with_href = doc.create_element("a").expect("create");
    doc.set_attribute(anchor_with_href, "href", "https://example.invalid")
        .expect("set");
    assert!(is_focusable_by_default(&doc, anchor_with_href.node_handle()).expect("check"));

    let plain_div = doc.create_element("div").expect("create");
    assert!(!is_focusable_by_default(&doc, plain_div.node_handle()).expect("check"));

    let text = doc.create_text("hello");
    assert!(
        !is_focusable_by_default(&doc, text.node_handle()).expect("check"),
        "non-element is never focusable"
    );
}

#[test]
fn nearest_focusable_walks_up_and_returns_none_when_nothing_qualifies() {
    let mut doc = Document::new();
    let button = make_element(&mut doc, "button");
    let inner_div = make_element(&mut doc, "div");
    let deepest_span = make_element(&mut doc, "span");
    let root = doc.root();
    append(&mut doc, root, button);
    append(&mut doc, button, inner_div);
    append(&mut doc, inner_div, deepest_span);

    assert_eq!(
        nearest_focusable(&doc, deepest_span).expect("check"),
        Some(button)
    );
    assert_eq!(
        nearest_focusable(&doc, button).expect("check"),
        Some(button)
    );

    let mut doc2 = Document::new();
    let plain = make_element(&mut doc2, "div");
    let root = doc2.root();
    append(&mut doc2, root, plain);
    assert_eq!(nearest_focusable(&doc2, plain).expect("check"), None);
}

#[test]
fn detaching_the_active_element_fires_blur_focusout_and_clears_focus_but_does_not_pick_a_new_target(
) {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let parent = make_element(&mut doc, "div");
    let a = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, parent);
    append(&mut doc, parent, a);

    let log = Rc::new(RefCell::new(Vec::new()));
    add_recorder(&mut registry, a, EventKind::Blur, 1, log.clone(), "blur@a");
    add_recorder(
        &mut registry,
        a,
        EventKind::FocusOut,
        2,
        log.clone(),
        "focusout@a",
    );

    focus(&mut doc, &mut registry, a).expect("focus a");
    log.borrow_mut().clear();

    doc.remove_child(parent, a).expect("detach a");
    registry
        .handle_node_changed(&mut doc, a, NodeChange::Detached)
        .expect("handle_node_changed");

    assert_eq!(*log.borrow(), vec!["blur@a", "focusout@a"]);
    assert_eq!(registry.active_element(), None);
}

#[test]
fn handle_node_changed_ignores_non_active_targets_and_non_detached_changes() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let a = make_element(&mut doc, "button");
    let b = make_element(&mut doc, "button");
    let container = root_container(&mut doc);
    append(&mut doc, container, a);
    append(&mut doc, container, b);

    let log = Rc::new(RefCell::new(Vec::new()));
    add_recorder(
        &mut registry,
        a,
        EventKind::Blur,
        1,
        log.clone(),
        "blur@a-should-not-fire",
    );

    focus(&mut doc, &mut registry, a).expect("focus a");
    log.borrow_mut().clear();

    // Detached notification for a non-active node: ignored.
    registry
        .handle_node_changed(&mut doc, b, NodeChange::Detached)
        .expect("ignored");
    assert!(log.borrow().is_empty());
    assert_eq!(registry.active_element(), Some(a));

    // Non-Detached change on the active node: ignored.
    registry
        .handle_node_changed(&mut doc, a, NodeChange::AttributesChanged)
        .expect("ignored");
    assert!(log.borrow().is_empty());
    assert_eq!(registry.active_element(), Some(a));
}

#[test]
fn handle_node_freed_clears_active_element_silently_with_no_events() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let parent = make_element(&mut doc, "div");
    let a = make_element(&mut doc, "button");
    let root = doc.root();
    append(&mut doc, root, parent);
    append(&mut doc, parent, a);

    let log = Rc::new(RefCell::new(Vec::new()));
    add_recorder(
        &mut registry,
        a,
        EventKind::Blur,
        1,
        log.clone(),
        "blur@a-should-not-fire",
    );

    focus(&mut doc, &mut registry, a).expect("focus a");
    log.borrow_mut().clear();

    doc.remove_child(parent, a).expect("detach a");
    doc.destroy_node(a).expect("destroy detached childless a");
    registry.handle_node_freed(a);

    assert!(
        log.borrow().is_empty(),
        "belt-and-suspenders clear fires no events"
    );
    assert_eq!(registry.active_element(), None);
}

#[test]
fn handle_document_teardown_clears_everything_in_one_call_with_no_events() {
    let mut doc = Document::new();
    let mut registry = EventTargetRegistry::for_document(&doc);
    let a = make_element(&mut doc, "button");
    let b = make_element(&mut doc, "button");
    let container = root_container(&mut doc);
    append(&mut doc, container, a);
    append(&mut doc, container, b);

    let log = Rc::new(RefCell::new(Vec::new()));
    add_recorder(
        &mut registry,
        a,
        EventKind::Blur,
        1,
        log.clone(),
        "blur@a-should-not-fire",
    );
    add_recorder(
        &mut registry,
        b,
        EventKind::Click,
        2,
        log.clone(),
        "click@b-should-not-fire",
    );

    focus(&mut doc, &mut registry, a).expect("focus a");
    log.borrow_mut().clear();

    let document_id = doc.id();
    registry.handle_document_teardown(document_id);

    assert!(log.borrow().is_empty());
    assert_eq!(registry.active_element(), None);
    assert_eq!(registry.listener_count(a), 0);
    assert_eq!(registry.listener_count(b), 0);
}
