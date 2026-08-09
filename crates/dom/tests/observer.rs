//! Wrapper-notification ordering/content tests: correct `NodeChange`
//! variant per mutation kind, never on a rejected mutation; `on_node_freed`
//! semantics distinct from mere detachment; bulk-teardown batching; and
//! reentrant `&self` observer callbacks.

mod support;

use std::sync::{Arc, Mutex};

use machina_dom::{Document, DocumentId, DomError, NodeChange, NodeHandle, WrapperObserver};
use support::RecordingObserver;

#[test]
fn on_node_changed_fires_the_correct_variant_for_each_mutation_kind_and_never_on_a_rejected_mutation(
) {
    let observer = RecordingObserver::new();
    let mut doc = Document::new();
    doc.set_wrapper_observer(Some(Box::new(observer.clone())));

    let root = doc.root();
    let element = doc.create_element("div").expect("create").node_handle();
    doc.append_child(root, element).expect("insert");
    assert_eq!(
        observer.changed.lock().expect("lock").last().copied(),
        Some((element, NodeChange::Inserted))
    );

    let element_typed = doc.as_element(element).expect("is element");
    doc.set_attribute(element_typed, "id", "x")
        .expect("set attribute");
    assert_eq!(
        observer.changed.lock().expect("lock").last().copied(),
        Some((element, NodeChange::AttributesChanged))
    );

    let text = doc.create_text("hi").node_handle();
    doc.append_child(element, text).expect("insert text");
    doc.set_text_data(text, "bye").expect("set text");
    assert_eq!(
        observer.changed.lock().expect("lock").last().copied(),
        Some((text, NodeChange::TextChanged))
    );

    doc.remove_child(root, element).expect("detach");
    assert_eq!(
        observer.changed.lock().expect("lock").last().copied(),
        Some((element, NodeChange::Detached))
    );

    let before = observer.changed.lock().expect("lock").len();
    let rejected = doc.append_child(element, element); // parent == new_child
    assert!(rejected.is_err());
    assert_eq!(
        observer.changed.lock().expect("lock").len(),
        before,
        "a rejected mutation must never notify"
    );
}

#[test]
fn on_node_freed_fires_once_per_freed_node_and_never_on_mere_detachment() {
    let observer = RecordingObserver::new();
    let mut doc = Document::new();
    doc.set_wrapper_observer(Some(Box::new(observer.clone())));

    let root = doc.root();
    let element = doc.create_element("div").expect("create").node_handle();
    doc.append_child(root, element).expect("attach");
    doc.remove_child(root, element).expect("detach");
    assert!(
        observer.freed.lock().expect("lock").is_empty(),
        "detach alone must not free"
    );

    doc.destroy_node(element).expect("reclaim");
    assert_eq!(observer.freed.lock().expect("lock").as_slice(), [element]);

    // A second reclaim attempt on the now-stale handle must fail, not fire
    // a second on_node_freed.
    assert_eq!(
        doc.destroy_node(element).unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(observer.freed.lock().expect("lock").len(), 1);
}

#[test]
fn bulk_teardown_fires_one_document_teardown_event_not_one_node_freed_per_node() {
    let observer = RecordingObserver::new();
    let mut doc = Document::new();
    doc.set_wrapper_observer(Some(Box::new(observer.clone())));

    for i in 0..25 {
        doc.create_element(&format!("n{i}")).expect("create");
    }
    doc.close();

    assert_eq!(observer.teardown.lock().expect("lock").len(), 1);
    assert!(observer.freed.lock().expect("lock").is_empty());
}

/// `WrapperObserver` methods all take `&self`, so `Document` itself never
/// needs any interior mutability (`RefCell`/lock) to support them. This
/// exercises a stateful observer that reads its own already-recorded state
/// again from inside its own later callback, demonstrating that reentrant
/// `&self` access compiles and behaves correctly (no deadlock, no borrow
/// conflict) with a plain, non-`Sync`-requiring `Document`.
#[derive(Clone)]
struct ReentrantCountingObserver {
    seen: Arc<Mutex<usize>>,
}

impl WrapperObserver for ReentrantCountingObserver {
    fn on_node_changed(&self, _handle: NodeHandle, _change: NodeChange) {
        let mut seen = self.seen.lock().expect("lock");
        *seen += 1;
        let now = *seen;
        drop(seen);
        // Reentrant read of this same observer's own state from inside its
        // own callback.
        assert_eq!(*self.seen.lock().expect("lock"), now);
    }

    fn on_node_freed(&self, _handle: NodeHandle) {}

    fn on_document_teardown(&self, _document: DocumentId) {}
}

#[test]
fn observer_callback_can_reentrantly_read_its_own_state_because_document_needs_no_interior_mutability(
) {
    let observer = ReentrantCountingObserver {
        seen: Arc::new(Mutex::new(0)),
    };
    let mut doc = Document::new();
    doc.set_wrapper_observer(Some(Box::new(observer.clone())));

    let root = doc.root();
    let container = doc
        .create_element("container")
        .expect("create")
        .node_handle();
    doc.append_child(root, container).expect("attach container");
    for i in 0..5 {
        let element = doc
            .create_element(&format!("n{i}"))
            .expect("create")
            .node_handle();
        doc.append_child(container, element).expect("attach");
    }

    assert_eq!(*observer.seen.lock().expect("lock"), 6);
}
