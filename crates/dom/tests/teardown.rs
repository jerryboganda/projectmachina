//! Bulk teardown / repeated create-destroy memory tests, plus
//! `destroy_node` single-slot reclamation (added per the M2-T05 security
//! review, finding 4: the ordinary `remove_child`-and-abandon path had no
//! reclaim mechanism in the original design).

mod support;

use machina_dom::{Document, DomError};
use support::RecordingObserver;

#[test]
fn close_reports_zero_memory_usage() {
    let mut doc = Document::new();
    doc.create_element("div").expect("create");
    doc.create_text("hello");
    assert!(doc.memory_usage().node_count > 0);

    doc.close();
    let usage = doc.memory_usage();
    assert_eq!(usage.node_count, 0);
    assert_eq!(usage.bytes_estimate, 0);
}

#[test]
fn every_pre_close_handle_uniformly_fails_document_closed() {
    let mut doc = Document::new();
    let element = doc.create_element("div").expect("create").node_handle();
    let text = doc.create_text("hi").node_handle();
    let comment = doc.create_comment("note");
    let root = doc.root();

    doc.close();

    for handle in [element, text, comment, root] {
        assert_eq!(doc.node(handle).unwrap_err(), DomError::DocumentClosed);
    }
}

#[test]
fn repeated_create_destroy_across_many_documents_shows_no_cross_document_accumulation() {
    const DOCUMENTS: usize = 20;
    const NODES_PER_DOCUMENT: usize = 50;

    let observer = RecordingObserver::new();
    for _ in 0..DOCUMENTS {
        let mut doc = Document::new();
        doc.set_wrapper_observer(Some(Box::new(observer.clone())));
        for i in 0..NODES_PER_DOCUMENT {
            doc.create_element(&format!("n{i}")).expect("create");
        }
        assert_eq!(
            doc.memory_usage().node_count,
            NODES_PER_DOCUMENT as u64 + 1 // + the document's own root node
        );
        doc.close();
        assert_eq!(doc.memory_usage().node_count, 0);
    }

    assert_eq!(observer.teardown.lock().expect("lock").len(), DOCUMENTS);
    // Bulk teardown must fire one on_document_teardown per document, not a
    // per-node on_node_freed for each of the (documents * nodes) nodes.
    assert!(observer.freed.lock().expect("lock").is_empty());
}

#[test]
fn drop_without_explicit_close_still_tears_down_and_fires_teardown_exactly_once() {
    let observer = RecordingObserver::new();
    {
        let mut doc = Document::new();
        doc.set_wrapper_observer(Some(Box::new(observer.clone())));
        doc.create_element("div").expect("create");
        // No explicit `doc.close()`: scope-drop must still tear down.
    }
    assert_eq!(observer.teardown.lock().expect("lock").len(), 1);
}

#[test]
fn close_is_idempotent_when_called_twice_on_a_still_live_document() {
    let observer = RecordingObserver::new();
    let mut doc = Document::new();
    doc.set_wrapper_observer(Some(Box::new(observer.clone())));
    doc.create_element("div").expect("create");

    doc.close();
    doc.close();

    assert_eq!(observer.teardown.lock().expect("lock").len(), 1);
    assert!(doc.is_closed());
}

#[test]
fn destroy_node_reclaims_a_detached_childless_node_and_notifies_on_node_freed() {
    let observer = RecordingObserver::new();
    let mut doc = Document::new();
    doc.set_wrapper_observer(Some(Box::new(observer.clone())));

    let root = doc.root();
    let element = doc.create_element("div").expect("create").node_handle();
    doc.append_child(root, element).expect("attach");

    // Still attached: destroy_node must refuse rather than silently
    // detaching-and-freeing.
    assert_eq!(
        doc.destroy_node(element).unwrap_err(),
        DomError::NodeStillAttached
    );

    doc.remove_child(root, element).expect("detach");
    let before = doc.memory_usage().node_count;
    doc.destroy_node(element)
        .expect("reclaim a detached, childless node");
    assert_eq!(doc.memory_usage().node_count, before - 1);
    assert_eq!(doc.node(element).unwrap_err(), DomError::StaleHandle);
    assert_eq!(observer.freed.lock().expect("lock").as_slice(), [element]);
}

#[test]
fn destroy_node_refuses_a_node_that_still_has_children() {
    let mut doc = Document::new();
    let parent = doc.create_element("div").expect("create").node_handle();
    let child = doc.create_text("hi").node_handle();
    doc.append_child(parent, child).expect("attach child");

    assert_eq!(
        doc.destroy_node(parent).unwrap_err(),
        DomError::NodeHasChildren
    );
}

/// M2-T05 security review, finding 4': `adopt_node` freeing a whole
/// subtree in the source document must fire one `on_node_freed` per freed
/// slot individually, not a single batched event (unlike `close()`, which
/// deliberately batches into one `on_document_teardown`).
#[test]
fn adopt_node_frees_every_subtree_slot_individually_not_as_one_batched_event() {
    let observer = RecordingObserver::new();
    let mut source = Document::new();
    source.set_wrapper_observer(Some(Box::new(observer.clone())));
    let parent = source.create_element("div").expect("create");
    let child_one = source.create_element("span").expect("create");
    let child_two = source.create_text("hi").node_handle();
    source
        .append_child(parent.node_handle(), child_one.node_handle())
        .expect("attach child_one");
    source
        .append_child(parent.node_handle(), child_two)
        .expect("attach child_two");

    let mut destination = Document::new();
    destination
        .adopt_node(&mut source, parent.node_handle())
        .expect("adopt");

    let freed = observer.freed.lock().expect("lock");
    assert_eq!(freed.len(), 3);
    assert!(freed.contains(&parent.node_handle()));
    assert!(freed.contains(&child_one.node_handle()));
    assert!(freed.contains(&child_two));
}
