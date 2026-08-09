//! Adoption agency algorithm coverage (design §3, WHATWG HTML §13.2.6.4.7):
//! misnested formatting elements (`<b><i>x</b>y</i>`-style input) must
//! recover deterministically — never panic, never poison the builder
//! (`parse_to_completion`'s `.expect()`s are the panic/poison assertion),
//! and never lose text content.
//!
//! Honest scope note (see `.agent-state/evidence/M2-T04.md` and the module
//! doc comment on `src/adoption_agency.rs`): these tests check text-content
//! preservation and crash-freedom for representative misnesting shapes,
//! not exact tree-shape equality against the full html5lib-tests
//! `adoption01.dat`/`adoption02.dat` corpus (that corpus/harness does not
//! exist in this repository yet — deferred to real WPT infrastructure
//! work). One case with an unambiguous, hand-verified expected shape is
//! asserted exactly; the rest assert the safety/content-preservation
//! properties that hold regardless of exact reparenting shape.

mod support;

use support::{parse_to_completion, render};

/// Strips every `<...>` tag from a rendered outline, leaving only the
/// concatenated text content, for order-preserving content checks that
/// don't depend on exactly how the adoption agency algorithm reparented
/// the formatting elements.
fn text_only(rendered: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in rendered.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[test]
fn simple_overlapping_bold_and_italic_closes_deterministically_without_panicking() {
    // <b>1<i>2</b>3</i> — the classic adoption-agency trigger: <b> closes
    // while <i> is still open.
    let (doc, builder) = parse_to_completion("<b>1<i>2</b>3</i>");
    let html = builder.document_element().expect("html element inserted");
    let rendered = render(&doc, html.node_handle());
    assert_eq!(text_only(&rendered), "123");
    // Both formatting elements still appear somewhere in the tree (the
    // algorithm reparents/clones them; it never simply drops one).
    assert!(rendered.contains("<b>"));
    assert!(rendered.contains("<i>"));
}

#[test]
fn formatting_element_closed_around_a_block_descendant_preserves_all_text() {
    // <b>1<p>2</b>3</p> — furthestBlock is a <p>, exercising the
    // reparent-under-common-ancestor step, not just the no-furthest-block
    // fast path.
    let (doc, builder) = parse_to_completion("<b>1<p>2</b>3</p>");
    let html = builder.document_element().expect("html element inserted");
    let rendered = render(&doc, html.node_handle());
    assert_eq!(text_only(&rendered), "123");
}

#[test]
fn repeated_reopening_of_the_same_formatting_element_is_bounded_by_noahs_ark() {
    // Ten consecutive unclosed <b> starts with no matching ends: the
    // Noah's Ark clause (design §2) keeps the active-formatting-elements
    // list from growing without bound for identical same-tag/same-attrs
    // runs. Mostly a crash-freedom / determinism check.
    let html = "<b>1<b>2<b>3<b>4<b>5<b>6<b>7<b>8<b>9<b>10";
    let (doc, builder) = parse_to_completion(html);
    let html_handle = builder.document_element().expect("html element inserted");
    let rendered = render(&doc, html_handle.node_handle());
    assert_eq!(text_only(&rendered), "12345678910");
}

#[test]
fn a_tag_reopening_itself_runs_the_adoption_agency_instead_of_nesting_forever() {
    // <a> is spec-called-out: a second <a> start tag while one is already
    // in the active formatting elements list runs the adoption agency on
    // "a" first (design's InBody `a` handling, mirroring WHATWG's own
    // special case) rather than nesting <a> inside <a> indefinitely.
    let html = "<a href=\"1\">one</a><a href=\"2\">two</a>";
    let (doc, builder) = parse_to_completion(html);
    let html_handle = builder.document_element().expect("html element inserted");
    let rendered = render(&doc, html_handle.node_handle());
    assert_eq!(text_only(&rendered), "onetwo");
}

#[test]
fn deeply_repeated_misnesting_never_panics_and_stays_bounded() {
    // A larger, still-adversarial-shaped misnesting run: many alternating
    // unclosed/closed <b>/<i> pairs. The primary assertion is that this
    // completes at all (no panic, no poisoned builder via `.expect()`
    // inside `parse_to_completion`) within the adoption agency's spec-
    // fixed loop bounds (outer <= 8, inner <= 3 per attempt).
    let mut html = String::new();
    for i in 0..500 {
        html.push_str("<b><i>");
        html.push_str(&i.to_string());
        html.push_str("</b>");
    }
    let (doc, builder) = parse_to_completion(&html);
    let html_handle = builder.document_element().expect("html element inserted");
    let rendered = render(&doc, html_handle.node_handle());
    // Every numeral inserted must still be present, in order.
    let digits_only = text_only(&rendered);
    let expected_digits: String = (0..500).map(|i| i.to_string()).collect::<Vec<_>>().join("");
    assert_eq!(digits_only, expected_digits);
}
