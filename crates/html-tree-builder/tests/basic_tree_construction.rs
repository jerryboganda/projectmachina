//! Acceptance criterion: "priority tree-construction fixtures match
//! normalized reference DOM." Hand-authored fixtures (see
//! `.agent-state/evidence/M2-T04.md` for why: the html5lib-tests
//! `.dat`-corpus/WPT harness infrastructure does not exist anywhere in this
//! repository yet — M2-T03 explicitly deferred it and no later task has
//! built it). Covers common html/head/body structure, implicit tag
//! insertion, paragraph/list auto-closing, and table structure including
//! foster parenting.

mod support;

use support::{parse_to_completion, render};

#[test]
fn minimal_document_round_trips_through_every_explicit_tag() {
    let (doc, builder) =
        parse_to_completion("<html><head><title>T</title></head><body><p>Hi</p></body></html>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head><title>T</title></head><body><p>Hi</p></body></html>"
    );
}

#[test]
fn bare_text_gets_implicit_html_head_body() {
    let (doc, builder) = parse_to_completion("Hello world");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body>Hello world</body></html>"
    );
}

#[test]
fn paragraph_auto_closes_before_a_block_level_start_tag() {
    let (doc, builder) = parse_to_completion("<p>one<div>two</div>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body><p>one</p><div>two</div></body></html>"
    );
}

#[test]
fn explicit_p_end_tag_still_closes_normally() {
    let (doc, builder) = parse_to_completion("<p>one</p><p>two</p>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body><p>one</p><p>two</p></body></html>"
    );
}

#[test]
fn list_items_implicitly_close_the_previous_sibling_li() {
    let (doc, builder) = parse_to_completion("<ul><li>a<li>b<li>c</ul>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body><ul><li>a</li><li>b</li><li>c</li></ul></body></html>"
    );
}

#[test]
fn headings_of_the_same_family_implicitly_close_each_other() {
    let (doc, builder) = parse_to_completion("<h1>one<h2>two</h2>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body><h1>one</h1><h2>two</h2></body></html>"
    );
}

#[test]
fn table_with_explicit_rows_gets_an_implicit_tbody() {
    let (doc, builder) = parse_to_completion("<table><tr><td>1</td></tr></table>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body><table><tbody><tr><td>1</td></tr></tbody></table></body></html>"
    );
}

#[test]
fn text_directly_inside_table_before_a_row_is_foster_parented_before_the_table() {
    let (doc, builder) = parse_to_completion("<table>stray<tr><td>x</td></tr></table>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body>stray<table><tbody><tr><td>x</td></tr></tbody></table></body></html>"
    );
}

#[test]
fn td_without_an_enclosing_tr_gets_an_implicit_row_and_body() {
    let (doc, builder) = parse_to_completion("<table><td>only-cell</td></table>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body><table><tbody><tr><td>only-cell</td></tr></tbody></table></body></html>"
    );
}

#[test]
fn attribute_values_survive_tree_construction() {
    let (doc, builder) = parse_to_completion("<a href=\"https://example.invalid\">link</a>");
    let html = builder.document_element().expect("html element inserted");
    let body_children = doc.children(html.node_handle()).unwrap();
    // <head> then <body>
    let body = body_children[1];
    let a_children = doc.children(body).unwrap();
    let a_handle = doc.as_element(a_children[0]).unwrap();
    assert_eq!(doc.tag_name(a_handle).unwrap(), "a");
    assert_eq!(
        doc.attribute(a_handle, "href").unwrap(),
        Some("https://example.invalid")
    );
}

#[test]
fn void_elements_do_not_swallow_following_content() {
    let (doc, builder) = parse_to_completion("<body><br><hr>after</body>");
    let html = builder.document_element().expect("html element inserted");
    assert_eq!(
        render(&doc, html.node_handle()),
        "<html><head></head><body><br></br><hr></hr>after</body></html>"
    );
}

#[test]
fn a_leading_comment_before_html_is_attached_to_the_document_not_dropped() {
    use machina_dom::NodeKind;
    let (doc, _builder) = parse_to_completion("<!-- top --><html><body>x</body></html>");
    let root_children = doc.children(doc.root()).unwrap();
    let has_comment = root_children
        .iter()
        .any(|&c| doc.node(c).map(|n| n.kind()) == Ok(NodeKind::Comment));
    assert!(has_comment, "expected a Comment child of the Document node");
}
