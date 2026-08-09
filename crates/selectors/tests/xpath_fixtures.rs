//! XPath subset fixtures (design §5): absolute/relative paths, `//`
//! abbreviation, the five in-scope axes, node tests, and simple predicates.

mod support;

use machina_dom::NodeHandle;
use machina_selectors::{evaluate_xpath, XPathItem};
use support::build_fixture;

fn node_items(items: &[XPathItem]) -> Vec<NodeHandle> {
    items
        .iter()
        .map(|item| match item {
            XPathItem::Node(handle) => *handle,
            XPathItem::Attribute { .. } => panic!("expected a Node item, got an Attribute item"),
        })
        .collect()
}

#[test]
fn absolute_child_path() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "/html/body/footer", None).unwrap();
    assert_eq!(
        node_items(&result.items),
        vec![fixture.footer.node_handle()]
    );
}

#[test]
fn descendant_abbreviation_matches_anywhere_in_the_document() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//li", None).unwrap();
    let expected: Vec<NodeHandle> = fixture.items.iter().map(|e| e.node_handle()).collect();
    assert_eq!(node_items(&result.items), expected);
}

#[test]
fn wildcard_node_test_matches_only_elements() {
    let mut document = machina_dom::Document::new();
    let root = document.root();
    let parent = support::el(&mut document, root, "div", &[]);
    support::text(
        &mut document,
        parent.node_handle(),
        "just text, not an element",
    );
    let child = support::el(&mut document, parent.node_handle(), "span", &[]);

    let result = evaluate_xpath(&document, "/div/*", None).unwrap();
    assert_eq!(node_items(&result.items), vec![child.node_handle()]);
}

#[test]
fn text_node_test() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//p/text()", None).unwrap();
    assert_eq!(result.items.len(), 1);
    match &result.items[0] {
        XPathItem::Node(handle) => {
            assert_eq!(fixture.document.text_data(*handle).unwrap(), "Hello");
        }
        other => panic!("expected a text Node item, got {other:?}"),
    }
}

#[test]
fn attribute_axis_produces_a_distinct_attribute_item() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//div/@id", None).unwrap();
    assert_eq!(result.items.len(), 1);
    match &result.items[0] {
        XPathItem::Attribute { owner, name, value } => {
            assert_eq!(*owner, fixture.main);
            assert_eq!(name, "id");
            assert_eq!(value, "main");
        }
        other => panic!("expected an Attribute item, got {other:?}"),
    }
}

#[test]
fn attribute_axis_on_a_missing_attribute_is_a_legitimate_empty_match() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//footer/@id", None).unwrap();
    assert!(result.items.is_empty());
}

#[test]
fn self_axis_abbreviation() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, ".", Some(fixture.main.node_handle())).unwrap();
    assert_eq!(node_items(&result.items), vec![fixture.main.node_handle()]);
}

#[test]
fn parent_axis_abbreviation() {
    let fixture = build_fixture();
    let result =
        evaluate_xpath(&fixture.document, "..", Some(fixture.intro.node_handle())).unwrap();
    assert_eq!(node_items(&result.items), vec![fixture.main.node_handle()]);
}

#[test]
fn positional_predicate() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//ul/li[2]", None).unwrap();
    assert_eq!(
        node_items(&result.items),
        vec![fixture.items[1].node_handle()]
    );
}

#[test]
fn last_predicate() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//ul/li[last()]", None).unwrap();
    assert_eq!(
        node_items(&result.items),
        vec![fixture.items[2].node_handle()]
    );
}

#[test]
fn attribute_exists_predicate() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//span[@data-role]", None).unwrap();
    assert_eq!(node_items(&result.items), vec![fixture.row1.node_handle()]);
}

#[test]
fn attribute_equals_predicate() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//span[@data-testid='row-2']", None).unwrap();
    assert_eq!(node_items(&result.items), vec![fixture.row2.node_handle()]);
}

#[test]
fn and_combined_predicate() {
    let fixture = build_fixture();
    let result = evaluate_xpath(
        &fixture.document,
        "//span[@data-testid='row-1' and @data-role]",
        None,
    )
    .unwrap();
    assert_eq!(node_items(&result.items), vec![fixture.row1.node_handle()]);

    let none = evaluate_xpath(
        &fixture.document,
        "//span[@data-testid='row-2' and @data-role]",
        None,
    )
    .unwrap();
    assert!(none.items.is_empty());
}

#[test]
fn explicit_axis_syntax_matches_the_abbreviated_form() {
    let fixture = build_fixture();
    let abbreviated = evaluate_xpath(&fixture.document, "//li", None).unwrap();
    let explicit = evaluate_xpath(&fixture.document, "/descendant::li", None).unwrap();
    assert_eq!(node_items(&abbreviated.items), node_items(&explicit.items));

    let child_abbrev = evaluate_xpath(&fixture.document, "/html/body", None).unwrap();
    let child_explicit =
        evaluate_xpath(&fixture.document, "/child::html/child::body", None).unwrap();
    assert_eq!(
        node_items(&child_abbrev.items),
        node_items(&child_explicit.items)
    );
}

#[test]
fn results_are_deduplicated_and_in_document_order_across_multiple_contexts() {
    let fixture = build_fixture();
    // Every span's descendant text, deduplicated even though spans overlap
    // in ancestry only trivially here — this exercises the merge/dedupe
    // path across a multi-context step.
    let result = evaluate_xpath(&fixture.document, "//div/span", None).unwrap();
    assert_eq!(
        node_items(&result.items),
        vec![fixture.row1.node_handle(), fixture.row2.node_handle()]
    );
}
