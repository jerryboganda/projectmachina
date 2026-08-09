//! Stale-handle and cross-document-handle failure tests. Every operation
//! must fail safely with a typed `DomError`, never panic, on a bad handle.

use machina_dom::{Document, DomError};

#[test]
fn stale_handle_after_free_fails_safely_across_every_operation() {
    let mut doc = Document::new();
    let element = doc.create_element("div").expect("create element");
    let handle = element.node_handle();
    doc.destroy_node(handle)
        .expect("destroy a detached, childless node");

    assert_eq!(doc.node(handle).unwrap_err(), DomError::StaleHandle);
    assert_eq!(doc.as_element(handle).unwrap_err(), DomError::StaleHandle);
    assert_eq!(doc.tag_name(element).unwrap_err(), DomError::StaleHandle);
    assert_eq!(
        doc.attribute(element, "id").unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(
        doc.set_attribute(element, "id", "x").unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(
        doc.remove_attribute(element, "id").unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(doc.text_data(handle).unwrap_err(), DomError::StaleHandle);
    assert_eq!(
        doc.set_text_data(handle, "x").unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(doc.children(handle).unwrap_err(), DomError::StaleHandle);
    assert_eq!(
        doc.insert_before(doc.root(), handle, None).unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(
        doc.append_child(doc.root(), handle).unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(
        doc.remove_child(doc.root(), handle).unwrap_err(),
        DomError::StaleHandle
    );
    let other = doc.create_text("x").node_handle();
    assert_eq!(
        doc.replace_child(doc.root(), other, handle).unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(doc.destroy_node(handle).unwrap_err(), DomError::StaleHandle);
    assert_eq!(
        doc.clone_node(handle, true).unwrap_err(),
        DomError::StaleHandle
    );
}

#[test]
fn cross_document_handle_is_rejected_even_when_index_and_generation_collide() {
    let mut doc_a = Document::new();
    let mut doc_b = Document::new();
    // Both documents allocate their first created node at the same
    // (index, generation) pair inside their own arenas; the document id
    // must still gate every dereference.
    let a_element = doc_a.create_element("div").expect("create").node_handle();
    let b_element = doc_b.create_element("span").expect("create").node_handle();

    assert_eq!(doc_b.node(a_element).unwrap_err(), DomError::WrongDocument);
    assert_eq!(doc_a.node(b_element).unwrap_err(), DomError::WrongDocument);
}

#[test]
fn cross_document_handle_in_a_mutation_method_is_rejected_and_leaves_the_tree_unchanged() {
    let mut doc_a = Document::new();
    let mut doc_b = Document::new();
    let a_element = doc_a.create_element("div").expect("create").node_handle();

    let before = doc_b.children(doc_b.root()).expect("read children");
    let result = doc_b.append_child(doc_b.root(), a_element);
    assert_eq!(result.unwrap_err(), DomError::WrongDocument);
    let after = doc_b.children(doc_b.root()).expect("read children");
    assert_eq!(before, after);
}

#[test]
fn generation_reuse_stales_the_old_handle_but_not_the_new_one() {
    let mut doc = Document::new();
    let first = doc.create_element("div").expect("create").node_handle();
    doc.destroy_node(first).expect("destroy");
    let second = doc.create_element("span").expect("create").node_handle();

    assert_eq!(doc.node(first).unwrap_err(), DomError::StaleHandle);
    assert!(doc.node(second).is_ok());
    assert_ne!(first, second);
}

#[test]
fn post_close_every_handle_fails_with_document_closed_not_stale_handle() {
    let mut doc = Document::new();
    let element = doc.create_element("div").expect("create").node_handle();
    let root = doc.root();
    doc.close();

    assert_eq!(doc.node(element).unwrap_err(), DomError::DocumentClosed);
    assert_eq!(doc.node(root).unwrap_err(), DomError::DocumentClosed);
    assert!(doc.is_closed());
}

#[test]
fn adopt_node_stales_every_descendant_handle_not_just_the_moved_root() {
    let mut source = Document::new();
    let parent = source.create_element("div").expect("create");
    let child = source.create_element("span").expect("create");
    source
        .append_child(parent.node_handle(), child.node_handle())
        .expect("attach child");

    let mut destination = Document::new();
    destination
        .adopt_node(&mut source, parent.node_handle())
        .expect("adopt");

    assert_eq!(
        source.node(parent.node_handle()).unwrap_err(),
        DomError::StaleHandle
    );
    assert_eq!(
        source.node(child.node_handle()).unwrap_err(),
        DomError::StaleHandle
    );
}

#[test]
fn wrong_kind_accessors_fail_safely_instead_of_returning_garbage() {
    let mut doc = Document::new();
    let text = doc.create_text("hello").node_handle();
    let element = doc.create_element("div").expect("create");

    // A kind-typed handle can only ever be produced by a checked
    // constructor/downcast, so `as_element` on a text handle is the only
    // way to observe the wrong-kind failure at the boundary.
    assert_eq!(doc.as_element(text).unwrap_err(), DomError::WrongKind);
    assert_eq!(
        doc.text_data(element.node_handle()).unwrap_err(),
        DomError::WrongKind
    );
}
