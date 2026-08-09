//! Fast-gate item (d): "revision-stamping and staleness detection matching
//! T10's pattern (extract -> mutate DOM -> re-extract -> assert revision
//! changed)." Mirrors
//! `crates/selectors/tests/revision_invalidation.rs`'s exact shape for
//! `QueryResult`/`XPathResult`, applied to this crate's three top-level
//! result types.

mod support;

use machina_dom::ElementHandle;
use machina_semantic::{extract_metadata, extract_semantic_index, generate_markdown};
use support::parse_html;

fn by_id(doc: &machina_dom::Document, id: &str) -> ElementHandle {
    machina_selectors::get_element_by_id(doc, id)
        .expect("query does not error")
        .unwrap_or_else(|| panic!("fixture must contain id={id:?}"))
}

#[test]
fn semantic_index_revision_strictly_increases_after_a_mutation_and_reflects_it() {
    let mut doc = parse_html("<html><body><h1 id=\"h\">Old</h1></body></html>");
    let first = extract_semantic_index(&doc).expect("first extraction succeeds");
    assert_eq!(first.headings[0].accessible_name.as_deref(), Some("Old"));

    let heading = by_id(&doc, "h");
    let text_child = doc
        .children(heading.node_handle())
        .expect("heading has children")[0];
    doc.set_text_data(text_child, "New")
        .expect("set_text_data on a live text node succeeds");

    let second = extract_semantic_index(&doc).expect("second extraction succeeds");
    assert!(
        second.revision.value() > first.revision.value(),
        "revision must strictly increase after a mutation"
    );
    assert_eq!(second.headings[0].accessible_name.as_deref(), Some("New"));

    // The first result is now detectably stale purely by comparing its own
    // stored revision against the document's current revision — the exact
    // mechanism named in the M2-T10 design doc.
    assert_ne!(first.revision, doc.revision());
    assert_eq!(second.revision, doc.revision());
}

#[test]
fn semantic_index_revision_reflects_a_newly_added_link() {
    let mut doc = parse_html("<html><body><p>No links yet</p></body></html>");
    let first = extract_semantic_index(&doc).expect("first extraction succeeds");
    assert_eq!(first.links.len(), 0);

    let body = machina_selectors::query_selector(&doc, "body")
        .expect("query does not error")
        .expect("body exists");
    let anchor = doc.create_element("a").expect("valid tag");
    doc.set_attribute(anchor, "href", "/new")
        .expect("set_attribute on a fresh element succeeds");
    let text = doc.create_text("New link").node_handle();
    doc.append_child(anchor.node_handle(), text)
        .expect("append_child succeeds");
    doc.append_child(body.node_handle(), anchor.node_handle())
        .expect("append_child succeeds");

    let second = extract_semantic_index(&doc).expect("second extraction succeeds");
    assert!(second.revision.value() > first.revision.value());
    assert_eq!(second.links.len(), 1);
    assert_eq!(second.links[0].href, "/new");
}

#[test]
fn markdown_revision_strictly_increases_and_reflects_a_mutation() {
    let mut doc = parse_html("<html><body><p id=\"p\">Before</p></body></html>");
    let first = generate_markdown(&doc).expect("markdown generation succeeds");
    assert!(first.markdown.contains("Before"));

    let paragraph = by_id(&doc, "p");
    let text_child = doc
        .children(paragraph.node_handle())
        .expect("paragraph has children")[0];
    doc.set_text_data(text_child, "After")
        .expect("set_text_data succeeds");

    let second = generate_markdown(&doc).expect("markdown generation succeeds");
    assert!(second.revision.value() > first.revision.value());
    assert!(second.markdown.contains("After"));
    assert!(!second.markdown.contains("Before"));
}

#[test]
fn metadata_revision_strictly_increases_and_reflects_a_mutation() {
    let mut doc = parse_html("<html><head><title>Old Title</title></head><body></body></html>");
    let first = extract_metadata(&doc).expect("metadata extraction succeeds");
    assert_eq!(first.title.as_deref(), Some("Old Title"));

    let title_element = machina_selectors::query_selector(&doc, "title")
        .expect("query does not error")
        .expect("title exists");
    let text_child = doc
        .children(title_element.node_handle())
        .expect("title has children")[0];
    doc.set_text_data(text_child, "New Title")
        .expect("set_text_data succeeds");

    let second = extract_metadata(&doc).expect("metadata extraction succeeds");
    assert!(second.revision.value() > first.revision.value());
    assert_eq!(second.title.as_deref(), Some("New Title"));
}

#[test]
fn a_no_op_extraction_still_reports_the_current_not_stale_revision() {
    let doc = parse_html("<html><body><p>Static</p></body></html>");
    let first = extract_semantic_index(&doc).expect("first extraction succeeds");
    let second = extract_semantic_index(&doc).expect("second extraction succeeds");
    assert_eq!(first.revision, second.revision);
    assert_eq!(first.revision, doc.revision());
}
