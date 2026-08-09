//! Acceptance criterion: "malformed nesting recovers deterministically" /
//! fast-gate item (2), "deep/adversarial nesting limit tests" (design §7,
//! specifically §7a/§7b). A deeply nested, never-closed run of the same
//! tag must fail closed at the tree-builder layer — recorded as
//! `Diagnostic::NestingLimitExceeded`, never a panic, and never a
//! `machina_dom::DomError::HierarchyViolation` escaping `Document` (which
//! would show up here as a poisoned builder / `Err` from `feed`/`finish`,
//! since `parse_to_completion` unconditionally `.expect()`s success).

mod support;

use machina_html_tree_builder::{Diagnostic, TreeBuilder, TreeBuilderLimits};
use support::{parse_to_completion, parse_to_completion_byte_at_a_time, render};

fn deeply_nested_divs(depth: usize) -> String {
    let mut html = String::from("<html><body>");
    for _ in 0..depth {
        html.push_str("<div>");
    }
    html
}

#[test]
fn deeply_nested_unclosed_divs_never_panic_and_record_a_diagnostic() {
    let html = deeply_nested_divs(20_000);
    let (doc, builder) = parse_to_completion(&html);

    assert!(
        builder
            .diagnostics()
            .iter()
            .any(|d| matches!(d, Diagnostic::NestingLimitExceeded { local_name, .. } if local_name == "div")),
        "expected at least one NestingLimitExceeded diagnostic for <div>"
    );

    // The DOM never grew past a small, bounded multiple of the configured
    // ceiling (well under the 20,000 divs requested) — the excess start
    // tags were never even passed to `Document::create_element_ns`.
    let usage = doc.memory_usage();
    let default_ceiling = TreeBuilderLimits::default().max_open_elements_depth as u64;
    assert!(
        usage.node_count < default_ceiling * 2,
        "node_count {} should stay well under twice the open-elements ceiling {}",
        usage.node_count,
        default_ceiling
    );
}

#[test]
fn nesting_ceiling_is_strictly_tighter_than_the_dom_hierarchy_violation_bound() {
    // Design §7a's own invariant, re-asserted from the acceptance-test
    // side (not just the unit test in `src/limits.rs`): the tree builder's
    // own ceiling always fires before `machina_dom::MAX_ANCESTOR_WALK`
    // could ever be reached by this crate's driving code.
    assert!(TreeBuilderLimits::default().max_open_elements_depth < machina_dom::MAX_ANCESTOR_WALK);
}

#[test]
fn parsing_never_poisons_the_builder_even_under_adversarial_nesting() {
    // `parse_to_completion` already `.expect()`s every `feed`/`finish`
    // call to succeed; reaching this line at all is the assertion that no
    // `TreeBuilderError::Internal` (poisoning) occurred while processing
    // 20,000 unclosed <div>s plus a trailing well-formed close.
    let mut html = deeply_nested_divs(20_000);
    html.push_str("</div></body></html>");
    let (_doc, _builder) = parse_to_completion(&html);
}

#[test]
fn text_after_the_ceiling_fires_is_still_inserted_not_dropped() {
    // Forward progress after the limit fires (design §7b: "the offending
    // token is treated as a failed open... parsing continues under the
    // current stack"): once the ceiling is hit the open-elements stack
    // stays pinned at its maximum (nothing in this fixture ever closes a
    // <div>), so a *new element* genuinely cannot be created again until
    // the stack shrinks back under the ceiling — but character data is
    // still inserted into whatever the current (capped-depth) node is,
    // rather than being silently discarded. That text reaching the tree at
    // all is the forward-progress guarantee this test checks.
    let mut html = deeply_nested_divs(5_000);
    html.push_str("still parses");
    let (doc, builder) = parse_to_completion(&html);
    let html_handle = builder.document_element().expect("html element inserted");
    let rendered = render(&doc, html_handle.node_handle());
    assert!(
        rendered.contains("still parses"),
        "text after the adversarial run should still be present: {rendered}"
    );
}

#[test]
fn closing_back_below_the_ceiling_allows_element_creation_to_resume() {
    // The stronger recovery claim: once enough proper end tags shrink the
    // open-elements stack back under the ceiling, ordinary element
    // creation works again — the ceiling is a live depth guard, not a
    // one-shot "parsing is now degraded forever" latch.
    let depth = 5_000;
    let mut html = deeply_nested_divs(depth);
    for _ in 0..depth {
        html.push_str("</div>");
    }
    html.push_str("<p>recovered</p>");
    let (doc, builder) = parse_to_completion(&html);
    let html_handle = builder.document_element().expect("html element inserted");
    let rendered = render(&doc, html_handle.node_handle());
    assert!(
        rendered.contains("<p>recovered</p>"),
        "element creation should resume once the stack is back under the ceiling: {rendered}"
    );
}

#[test]
fn nesting_limit_outcome_is_identical_whole_vs_byte_at_a_time() {
    // Chunk-boundary equivalence, extended one layer up from M2-T03's own
    // tokenizer-level guarantee (design §8): the same document fed whole
    // vs one byte per `feed()` call must produce the same final node count
    // and the same number of recorded diagnostics.
    let html = deeply_nested_divs(6_000);
    let (doc_whole, builder_whole) = parse_to_completion(&html);
    let (doc_chunked, builder_chunked) = parse_to_completion_byte_at_a_time(&html);

    assert_eq!(
        doc_whole.memory_usage().node_count,
        doc_chunked.memory_usage().node_count
    );
    assert_eq!(
        builder_whole.diagnostics().len(),
        builder_chunked.diagnostics().len()
    );
}

#[test]
fn a_custom_lower_ceiling_is_honored_and_still_deterministic() {
    let limits = TreeBuilderLimits {
        max_open_elements_depth: 50,
        ..TreeBuilderLimits::default()
    };
    let mut doc = machina_dom::Document::new();
    let mut tokenizer = machina_html::Tokenizer::new(machina_html::TokenizerLimits::default());
    let mut builder = TreeBuilder::with_limits(false, limits);
    let html = deeply_nested_divs(500);
    let mut outcome = builder
        .feed(&mut doc, &mut tokenizer, html.as_bytes())
        .unwrap();
    loop {
        outcome = match outcome {
            machina_html_tree_builder::TreeBuilderOutcome::NeedsMoreInput => {
                builder.finish(&mut doc, &mut tokenizer).unwrap()
            }
            machina_html_tree_builder::TreeBuilderOutcome::ScriptCheckpoint(_) => builder
                .resume_after_script(&mut doc, &mut tokenizer)
                .unwrap(),
            machina_html_tree_builder::TreeBuilderOutcome::Done => break,
        };
    }
    assert!(builder
        .diagnostics()
        .iter()
        .any(|d| matches!(d, Diagnostic::NestingLimitExceeded { .. })));
    // html + body + up to 50 divs, comfortably under 500.
    assert!(doc.memory_usage().node_count < 100);
}
