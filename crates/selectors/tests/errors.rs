//! Fast-gate item: "invalid expressions never crash or silently match" —
//! the three-way error split (design §6), with explicit, separately-named
//! tests distinguishing:
//! 1. a **legitimate empty match** (`Ok(vec![])`, not an error at all);
//! 2. **malformed syntax** (always `InvalidSelector`/`InvalidXPath`, staged
//!    strictly before any tree walk);
//! 3. a **valid-but-out-of-scope construct** (always `UnsupportedFeature`,
//!    never silently dropped/no-op'd, never conflated with case 2).
//!
//! `:hover` (case 3) is explicitly tested against `:not(` unterminated
//! (case 2) to prove the split is real, not accidental, per the task's own
//! acceptance wording.

mod support;

use machina_selectors::{css, evaluate_xpath, query_selector_all, QueryError};
use support::build_fixture;

// ---- 1. legitimate empty match: Ok(vec![]), never an error ---------------

#[test]
fn legitimate_empty_match_is_ok_empty_not_an_error() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "article.does-not-exist");
    assert!(
        result.is_ok(),
        "a well-formed selector matching nothing must be Ok, not Err"
    );
    assert!(result.unwrap().elements.is_empty());
}

#[test]
fn legitimate_empty_xpath_match_is_ok_empty_not_an_error() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//article", None);
    assert!(
        result.is_ok(),
        "a well-formed xpath matching nothing must be Ok, not Err"
    );
    assert!(result.unwrap().items.is_empty());
}

// ---- 2. malformed syntax: always a typed parse error, staged before any
//         matching, never a panic and never a partial match --------------

#[test]
fn unterminated_not_pseudo_class_is_invalid_selector_not_unsupported() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "div:not(");
    match result {
        Err(QueryError::InvalidSelector { .. }) => {}
        other => panic!("expected InvalidSelector for unterminated ':not(', got {other:?}"),
    }
}

#[test]
fn unterminated_attribute_bracket_is_invalid_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[data-testid");
    assert!(matches!(result, Err(QueryError::InvalidSelector { .. })));
}

#[test]
fn dangling_combinator_is_invalid_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "div >");
    assert!(matches!(result, Err(QueryError::InvalidSelector { .. })));
}

#[test]
fn bare_combinator_with_no_selector_at_all_is_invalid_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, ">");
    assert!(matches!(result, Err(QueryError::InvalidSelector { .. })));
}

#[test]
fn empty_selector_string_is_invalid_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "");
    assert!(matches!(result, Err(QueryError::InvalidSelector { .. })));
}

#[test]
fn malformed_nth_child_argument_is_invalid_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li:nth-child(abc)");
    assert!(matches!(result, Err(QueryError::InvalidSelector { .. })));
}

#[test]
fn unterminated_string_literal_is_invalid_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[data-testid=\"row-1]");
    assert!(matches!(result, Err(QueryError::InvalidSelector { .. })));
}

#[test]
fn malformed_syntax_never_panics_on_adversarial_bytes() {
    let fixture = build_fixture();
    let adversarial_inputs = [
        "\u{0}",
        "[[[[[[[[[[",
        "))))))))))",
        "::::::",
        "div[",
        ".:not(#[",
        "a~~~~~b",
        "[a=",
    ];
    for input in adversarial_inputs {
        // The only contract under test here is "never panics"; whether it
        // is Ok or Err is not asserted per-input, just that it terminates
        // with a typed result.
        let _ = query_selector_all(&fixture.document, input);
    }
}

#[test]
fn xpath_malformed_syntax_is_invalid_xpath() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//div[", None);
    assert!(matches!(result, Err(QueryError::InvalidXPath { .. })));
}

#[test]
fn xpath_unknown_node_test_function_is_invalid_xpath() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//bogus-function()", None);
    assert!(matches!(result, Err(QueryError::InvalidXPath { .. })));
}

// ---- 3. valid-but-out-of-scope construct: always UnsupportedFeature,
//         never silently dropped and never conflated with malformed syntax

#[test]
fn hover_pseudo_class_is_unsupported_feature_not_invalid_selector() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "a:hover");
    match result {
        Err(QueryError::UnsupportedFeature { ref feature, .. }) => {
            assert_eq!(feature, ":hover");
        }
        other => panic!("expected UnsupportedFeature for ':hover', got {other:?}"),
    }
}

#[test]
fn hover_and_unterminated_not_produce_distinct_error_variants() {
    // The core of the three-way split, stated as one direct comparison:
    // a *valid* pseudo-class this crate does not implement must never be
    // classified the same way as *malformed* syntax.
    let fixture = build_fixture();
    let hover = query_selector_all(&fixture.document, "a:hover");
    let unterminated_not = query_selector_all(&fixture.document, "div:not(");
    assert!(matches!(hover, Err(QueryError::UnsupportedFeature { .. })));
    assert!(matches!(
        unterminated_not,
        Err(QueryError::InvalidSelector { .. })
    ));
    assert_ne!(
        std::mem::discriminant(hover.as_ref().unwrap_err()),
        std::mem::discriminant(unterminated_not.as_ref().unwrap_err())
    );
}

#[test]
fn has_pseudo_class_with_argument_is_unsupported_feature() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "div:has(span)");
    assert!(matches!(result, Err(QueryError::UnsupportedFeature { .. })));
}

#[test]
fn first_of_type_is_unsupported_feature() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "li:first-of-type");
    assert!(matches!(result, Err(QueryError::UnsupportedFeature { .. })));
}

#[test]
fn attribute_case_insensitivity_flag_is_unsupported_feature() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "[data-testid=\"ROW-1\" i]");
    assert!(matches!(result, Err(QueryError::UnsupportedFeature { .. })));
}

#[test]
fn pseudo_element_is_unsupported_feature() {
    let fixture = build_fixture();
    let result = query_selector_all(&fixture.document, "p::before");
    assert!(matches!(result, Err(QueryError::UnsupportedFeature { .. })));
}

#[test]
fn xpath_following_sibling_axis_is_unsupported_feature() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//li/following-sibling::li", None);
    assert!(matches!(result, Err(QueryError::UnsupportedFeature { .. })));
}

#[test]
fn xpath_contains_function_is_unsupported_feature() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "//li[contains(text(),'One')]", None);
    assert!(matches!(result, Err(QueryError::UnsupportedFeature { .. })));
}

// ---- relative XPath without a context node: a distinct, dedicated typed
//      error, never a silent default to the document root -----------------

#[test]
fn relative_xpath_without_context_node_is_a_dedicated_error_not_a_silent_default() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "li", None);
    assert!(matches!(result, Err(QueryError::ContextNodeRequired)));
}

#[test]
fn relative_xpath_with_context_node_succeeds() {
    let fixture = build_fixture();
    let result = evaluate_xpath(&fixture.document, "li", Some(fixture.list.node_handle())).unwrap();
    assert_eq!(result.items.len(), 3);
}

// ---- adversarial nesting fails closed with TooComplex, never unbounded
//      recursion / stack overflow -----------------------------------------

#[test]
fn deeply_nested_not_fails_closed_with_too_complex_not_a_stack_overflow() {
    let fixture = build_fixture();
    let mut selector = String::from("div");
    for _ in 0..500 {
        selector = format!("div:not({selector})");
    }
    let result = query_selector_all(&fixture.document, &selector);
    assert!(matches!(result, Err(QueryError::TooComplex { .. })));
}

#[test]
fn excessively_long_combinator_chain_fails_closed_with_too_complex() {
    let fixture = build_fixture();
    let mut selector = String::from("div");
    for _ in 0..500 {
        selector.push_str(" > div");
    }
    let result = query_selector_all(&fixture.document, &selector);
    assert!(matches!(result, Err(QueryError::TooComplex { .. })));
}

// ---- css::parse_selector surfaces the same typed errors directly, for a
//      caller that wants to compile once and match many elements ---------

#[test]
fn compiled_selector_api_surfaces_the_same_typed_errors() {
    let result = css::parse_selector("div:not(");
    assert!(matches!(result, Err(QueryError::InvalidSelector { .. })));
    let result = css::parse_selector("a:hover");
    assert!(matches!(result, Err(QueryError::UnsupportedFeature { .. })));
    let result = css::parse_selector("div.card");
    assert!(result.is_ok());
}
