//! Fast-gate item: "query order and live document revisions are correct" —
//! document-order and combinator-backtracking correctness.

mod support;

use machina_selectors::query_selector_all;
use support::{build_fixture, el};

#[test]
fn results_are_in_strict_document_order_not_match_discovery_order() {
    let fixture = build_fixture();
    // Matches are discovered as: row1/row2 (children of main, visited before
    // ul's own descendants in a naive right-to-left-only walk would still
    // need document order) and every li. Assert the combined selector list
    // returns elements in document order regardless of which branch of the
    // OR/tree structure produced them.
    let result = query_selector_all(&fixture.document, "span, li").unwrap();
    let expected = vec![
        fixture.items[0],
        fixture.items[1],
        fixture.items[2],
        fixture.row1,
        fixture.row2,
    ];
    assert_eq!(result.elements, expected);
}

#[test]
fn descendant_combinator_backtracks_across_multiple_ancestor_candidates() {
    // <a><b><c><target class="x"/></c></b></a> plus a second, shallower
    // "b.x" ancestor further up that would falsely satisfy a naive
    // left-to-right walk if backtracking were broken.
    let mut document = machina_dom::Document::new();
    let root = document.root();
    let outer = el(&mut document, root, "div", &[("class", "x")]);
    let a = el(&mut document, outer.node_handle(), "a", &[]);
    let b = el(&mut document, a.node_handle(), "div", &[]);
    let target = el(
        &mut document,
        b.node_handle(),
        "span",
        &[("class", "target")],
    );

    // "div.x span.target": the innermost ancestor chain is
    // div(no class) -> div(no class) -> div.x(outer) -> ... — the matcher
    // must walk past the immediate non-matching parent(s) and keep trying
    // ancestors until it reaches `outer`, backtracking rather than giving up
    // after the first failed ancestor.
    let result = query_selector_all(&document, "div.x span.target").unwrap();
    assert_eq!(result.elements, vec![target]);
    let _ = b; // silence unused-binding lint while keeping the tree shape explicit
}

#[test]
fn general_sibling_combinator_backtracks_across_multiple_candidates() {
    // <ul><li class="a"/><li/><li class="target"/></ul> — "li.a ~
    // li.target" must walk past the middle, non-matching `li` and keep
    // trying earlier siblings.
    let mut document = machina_dom::Document::new();
    let root = document.root();
    let list = el(&mut document, root, "ul", &[]);
    let _first = el(&mut document, list.node_handle(), "li", &[("class", "a")]);
    let _middle = el(&mut document, list.node_handle(), "li", &[]);
    let target = el(
        &mut document,
        list.node_handle(),
        "li",
        &[("class", "target")],
    );

    let result = query_selector_all(&document, "li.a ~ li.target").unwrap();
    assert_eq!(result.elements, vec![target]);
}

#[test]
fn adjacent_sibling_combinator_requires_immediate_predecessor_not_any_earlier_one() {
    let mut document = machina_dom::Document::new();
    let root = document.root();
    let list = el(&mut document, root, "ul", &[]);
    let _first = el(&mut document, list.node_handle(), "li", &[("class", "a")]);
    let _middle = el(&mut document, list.node_handle(), "li", &[]);
    let _target = el(
        &mut document,
        list.node_handle(),
        "li",
        &[("class", "target")],
    );

    // "li.a + li.target" must fail: `.a` is not the *immediate* predecessor
    // of `.target` (an unrelated `li` sits between them).
    let result = query_selector_all(&document, "li.a + li.target").unwrap();
    assert!(result.elements.is_empty());
}

#[test]
fn child_combinator_backtracking_does_not_cross_a_non_matching_intermediate_generation() {
    // <div class="x"><section><span class="target"/></section></div> — a
    // child combinator must reject via the intermediate `section`, even
    // though a *descendant* combinator would have matched.
    let mut document = machina_dom::Document::new();
    let root = document.root();
    let outer = el(&mut document, root, "div", &[("class", "x")]);
    let section = el(&mut document, outer.node_handle(), "section", &[]);
    let target = el(
        &mut document,
        section.node_handle(),
        "span",
        &[("class", "target")],
    );

    let child_result = query_selector_all(&document, "div.x > span.target").unwrap();
    assert!(child_result.elements.is_empty());
    let descendant_result = query_selector_all(&document, "div.x span.target").unwrap();
    assert_eq!(descendant_result.elements, vec![target]);
}
