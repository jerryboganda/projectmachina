//! Priority CSS selector fixtures (fast gate item: "selector/XPath
//! fixtures"). Hand-written, since no WPT/html5lib-tests vendoring
//! infrastructure exists anywhere in this repository yet (confirmed absent
//! by M2-T03/M2-T04's own evidence docs) — deferred, tracked infrastructure
//! work, not a corner cut; see `.agent-state/evidence/M2-T10.md`.

mod support;

use machina_selectors::query_selector_all;
use support::build_fixture;

fn tags(document: &machina_dom::Document, elements: &[machina_dom::ElementHandle]) -> Vec<String> {
    elements
        .iter()
        .map(|&e| document.tag_name(e).unwrap().to_string())
        .collect()
}

#[test]
fn universal_selector_matches_every_element() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "*").unwrap();
    // html, body, div, p, ul, li*3, span*2, footer = 11
    assert_eq!(result.elements.len(), 11);
}

#[test]
fn type_selector_matches_by_tag_name() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li").unwrap();
    assert_eq!(result.elements, fixture.items);
}

#[test]
fn id_selector_matches_exactly_one_element() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "#main").unwrap();
    assert_eq!(result.elements, vec![fixture.main]);
}

#[test]
fn class_selector_matches_by_whitespace_separated_word() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, ".item").unwrap();
    assert_eq!(result.elements, fixture.items);

    let selected = query_selector_all(&fixture.document, ".selected").unwrap();
    assert_eq!(selected.elements, vec![fixture.items[1]]);
}

#[test]
fn attribute_presence_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[data-testid]").unwrap();
    assert_eq!(result.elements, vec![fixture.row1, fixture.row2]);
}

#[test]
fn attribute_equals_operator() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[data-testid=\"row-1\"]").unwrap();
    assert_eq!(result.elements, vec![fixture.row1]);
}

#[test]
fn attribute_prefix_match_operator() {
    let fixture = build_fixture();
    // The exact "automation selector" shape named in the fast gate.
    let result = query_selector_all(&fixture.document, "[data-testid^=\"row-\"]").unwrap();
    assert_eq!(result.elements, vec![fixture.row1, fixture.row2]);
}

#[test]
fn attribute_suffix_match_operator() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[data-testid$=\"1\"]").unwrap();
    assert_eq!(result.elements, vec![fixture.row1]);
}

#[test]
fn attribute_substring_match_operator() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[data-testid*=\"ow\"]").unwrap();
    assert_eq!(result.elements, vec![fixture.row1, fixture.row2]);
}

#[test]
fn attribute_includes_match_operator() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[class~=\"selected\"]").unwrap();
    assert_eq!(result.elements, vec![fixture.items[1]]);
}

#[test]
fn attribute_dash_match_operator() {
    let mut document = machina_dom::Document::new();
    let root = document.root();
    let element = support::el(&mut document, root, "div", &[("lang", "en-US")]);
    let result = query_selector_all(&document, "[lang|=\"en\"]").unwrap();
    assert_eq!(result.elements, vec![element]);
    let no_match = query_selector_all(&document, "[lang|=\"de\"]").unwrap();
    assert!(no_match.elements.is_empty());
}

#[test]
fn descendant_combinator() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "div li").unwrap();
    assert_eq!(result.elements, fixture.items);
}

#[test]
fn child_combinator_does_not_match_grandchildren() {
    let fixture = build_fixture();
    let direct = query_selector_all(&fixture.document, "main > li").unwrap();
    assert!(direct.elements.is_empty());
    let real = query_selector_all(&fixture.document, "ul > li").unwrap();
    assert_eq!(real.elements, fixture.items);
    let via_div = query_selector_all(&fixture.document, "div > ul").unwrap();
    assert_eq!(via_div.elements, vec![fixture.list]);
}

#[test]
fn adjacent_sibling_combinator() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li + li").unwrap();
    assert_eq!(result.elements, vec![fixture.items[1], fixture.items[2]]);
}

#[test]
fn general_sibling_combinator() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li ~ li").unwrap();
    assert_eq!(result.elements, vec![fixture.items[1], fixture.items[2]]);

    let from_span = query_selector_all(&fixture.document, "ul ~ span").unwrap();
    assert_eq!(from_span.elements, vec![fixture.row1, fixture.row2]);
}

#[test]
fn selector_list_is_logical_or() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "footer, #main").unwrap();
    assert_eq!(result.elements, vec![fixture.main, fixture.footer]);
}

#[test]
fn compound_selector_requires_all_simple_selectors() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li.selected").unwrap();
    assert_eq!(result.elements, vec![fixture.items[1]]);
    let none = query_selector_all(&fixture.document, "p.selected").unwrap();
    assert!(none.elements.is_empty());
}

#[test]
fn structural_pseudo_first_last_only_child() {
    let fixture = build_fixture();
    let first = query_selector_all(&fixture.document, "li:first-child").unwrap();
    assert_eq!(first.elements, vec![fixture.items[0]]);
    let last = query_selector_all(&fixture.document, "li:last-child").unwrap();
    assert_eq!(last.elements, vec![fixture.items[2]]);
    let only = query_selector_all(&fixture.document, "ul:only-child").unwrap();
    assert!(only.elements.is_empty()); // ul has siblings (p, span, span)
    let only_body_child = query_selector_all(&fixture.document, "body > div:only-child").unwrap();
    assert!(only_body_child.elements.is_empty()); // div has sibling footer
}

#[test]
fn structural_pseudo_empty() {
    let fixture = build_fixture();
    let empty = query_selector_all(&fixture.document, "footer:empty").unwrap();
    assert_eq!(empty.elements, vec![fixture.footer]);
    let not_empty = query_selector_all(&fixture.document, "ul:empty").unwrap();
    assert!(not_empty.elements.is_empty());
}

#[test]
fn structural_pseudo_root() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, ":root").unwrap();
    assert_eq!(result.elements, vec![fixture.html]);
}

#[test]
fn structural_pseudo_nth_child_odd_even_and_an_plus_b() {
    let fixture = build_fixture();
    let odd = query_selector_all(&fixture.document, "li:nth-child(odd)").unwrap();
    assert_eq!(odd.elements, vec![fixture.items[0], fixture.items[2]]);
    let even = query_selector_all(&fixture.document, "li:nth-child(even)").unwrap();
    assert_eq!(even.elements, vec![fixture.items[1]]);
    let second = query_selector_all(&fixture.document, "li:nth-child(2)").unwrap();
    assert_eq!(second.elements, vec![fixture.items[1]]);
    let from_2 = query_selector_all(&fixture.document, "li:nth-child(n+2)").unwrap();
    assert_eq!(from_2.elements, vec![fixture.items[1], fixture.items[2]]);
}

#[test]
fn nth_child_negative_coefficient_and_trailing_minus_forms_round_trip_through_the_real_tokenizer() {
    // These exercise the full selector-text -> tokenizer -> raw-substring
    // reconstruction -> parse_nth pipeline (not just the direct
    // css::pseudo::parse_nth unit tests), including a leading '-' before
    // 'n' and a bare "n-1" form where the sign merges into a single
    // identifier-like token.
    let fixture = build_fixture();
    // "-n+3": items 1..=3 all satisfy position <= 3 for a=-1,b=3.
    let result = query_selector_all(&fixture.document, "li:nth-child(-n+3)").unwrap();
    assert_eq!(result.elements, fixture.items);
    // "n-1": a=1,b=-1 -> position >= 1, i.e. every position (n=1 gives 0,
    // n=2 gives 1, ...) — still matches every li here since positions start
    // at 1 and diff=position-(-1) is always >=0 and divisible by 1.
    let result = query_selector_all(&fixture.document, "li:nth-child(n-1)").unwrap();
    assert_eq!(result.elements, fixture.items);
    // "3n": only position 3 (a=3,b=0) within this 3-item list.
    let result = query_selector_all(&fixture.document, "li:nth-child(3n)").unwrap();
    assert_eq!(result.elements, vec![fixture.items[2]]);
}

#[test]
fn structural_pseudo_nth_last_child() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li:nth-last-child(1)").unwrap();
    assert_eq!(result.elements, vec![fixture.items[2]]);
}

#[test]
fn negation_pseudo_class() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li:not(.selected)").unwrap();
    assert_eq!(result.elements, vec![fixture.items[0], fixture.items[2]]);
}

#[test]
fn combined_realistic_automation_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(
        &fixture.document,
        "div.container > span[data-testid^=\"row-\"]",
    )
    .unwrap();
    assert_eq!(result.elements, vec![fixture.row1, fixture.row2]);
    assert_eq!(
        tags(&fixture.document, &result.elements),
        vec!["span", "span"]
    );
}
