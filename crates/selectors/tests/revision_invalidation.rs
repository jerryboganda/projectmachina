//! Fast-gate item: the revision-invalidation contract — query → mutate →
//! requery → assert the result's `Revision` strictly increased and the
//! requeried result reflects the post-mutation tree, exactly per design §4b
//! ("staleness detection is plain `result.revision != document.revision()`,
//! never guessed/implicit").

mod support;

use machina_selectors::{evaluate_xpath, get_element_by_id, query_selector_all};
use support::{build_fixture, el};

#[test]
fn query_result_revision_matches_document_revision_at_query_time() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li").unwrap();
    assert_eq!(result.revision, fixture.document.revision());
}

#[test]
fn mutation_strictly_increases_revision_and_stale_result_is_mechanically_detectable() {
    let mut fixture = build_fixture();
    let first = query_selector_all(&fixture.document, "li").unwrap();
    assert_eq!(first.elements.len(), 3);
    let revision_before = first.revision;

    // Mutate: add a fourth <li>.
    let new_item = el(&mut fixture.document, fixture.list.node_handle(), "li", &[]);

    let revision_after_mutation = fixture.document.revision();
    assert!(
        revision_after_mutation > revision_before,
        "revision must strictly increase after a real mutation"
    );

    // The old QueryResult is now mechanically detectable as stale, exactly
    // via `result.revision != document.revision()` — no guessing.
    assert_ne!(first.revision, fixture.document.revision());

    // Requery: the new result reflects the post-mutation tree (4 items now)
    // and self-stamps the new revision.
    let second = query_selector_all(&fixture.document, "li").unwrap();
    assert_eq!(second.elements.len(), 4);
    assert!(second.elements.contains(&new_item));
    assert_eq!(second.revision, fixture.document.revision());
    assert!(second.revision > first.revision);
}

#[test]
fn removing_a_matched_element_is_reflected_on_requery() {
    let mut fixture = build_fixture();
    let first = query_selector_all(&fixture.document, ".selected").unwrap();
    assert_eq!(first.elements, vec![fixture.items[1]]);

    fixture
        .document
        .remove_child(fixture.list.node_handle(), fixture.items[1].node_handle())
        .unwrap();

    let second = query_selector_all(&fixture.document, ".selected").unwrap();
    assert!(second.elements.is_empty());
    assert!(second.revision > first.revision);
}

#[test]
fn attribute_mutation_is_reflected_on_requery() {
    let mut fixture = build_fixture();
    let before = query_selector_all(&fixture.document, "[data-testid=\"row-1\"]").unwrap();
    assert_eq!(before.elements, vec![fixture.row1]);

    fixture
        .document
        .set_attribute(fixture.row1, "data-testid", "row-renamed")
        .unwrap();

    let after = query_selector_all(&fixture.document, "[data-testid=\"row-1\"]").unwrap();
    assert!(after.elements.is_empty());
    assert!(after.revision > before.revision);

    let renamed = query_selector_all(&fixture.document, "[data-testid=\"row-renamed\"]").unwrap();
    assert_eq!(renamed.elements, vec![fixture.row1]);
}

#[test]
fn a_query_that_touches_nothing_still_reports_the_current_revision_unstale() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "nonexistent-tag").unwrap();
    assert!(result.elements.is_empty());
    assert_eq!(result.revision, fixture.document.revision());
}

#[test]
fn get_element_by_id_also_carries_correct_live_semantics_across_mutation() {
    let mut fixture = build_fixture();
    assert_eq!(
        get_element_by_id(&fixture.document, "main").unwrap(),
        Some(fixture.main)
    );
    fixture
        .document
        .remove_attribute(fixture.main, "id")
        .unwrap();
    assert_eq!(get_element_by_id(&fixture.document, "main").unwrap(), None);
}

#[test]
fn xpath_result_revision_also_follows_the_same_contract() {
    let mut fixture = build_fixture();
    let before = evaluate_xpath(&fixture.document, "//li", None).unwrap();
    assert_eq!(before.items.len(), 3);
    assert_eq!(before.revision, fixture.document.revision());

    let _new_item = el(&mut fixture.document, fixture.list.node_handle(), "li", &[]);

    let after = evaluate_xpath(&fixture.document, "//li", None).unwrap();
    assert_eq!(after.items.len(), 4);
    assert!(after.revision > before.revision);
}
