//! Foreign content (SVG/MathML, WHATWG HTML §13.2.6.2, design §4).
//!
//! MVP scope, documented explicitly (see `.agent-state/evidence/M2-T04.md`):
//! elements inside `<svg>`/`<math>` are created via `create_element_ns`
//! with the correct [`machina_dom::Namespace`] and a small SVG tag-case
//! adjustment table (the handful of camelCase SVG tag names WHATWG
//! requires, e.g. `foreignObject`); namespaced foreign attributes
//! (`xlink:href` and friends) are stored as one compound interned
//! attribute name rather than a true `(namespace, local name)` pair, since
//! `machina_dom::AttributeMap` is namespace-agnostic (M2-T05 design note,
//! carried into the M2-T04 design as an explicit MVP simplification).
//! HTML-integration-point / MathML-text-integration-point re-entry into
//! ordinary HTML insertion-mode rules mid-foreign-subtree is **not**
//! implemented in this pass — entering foreign content processes tokens
//! under simplified foreign rules until the matching foreign root's end
//! tag closes it, which is spec-accurate for the common case (opaque SVG/
//! MathML subtrees) but deviates for the integration-point edge cases.
//! Documented as a real, tracked gap, not a silent omission.

/// SVG tag names WHATWG requires to be case-adjusted from what an HTML
/// tokenizer (which lowercases tag names) hands back. Representative
/// subset covering the common elements likely to appear in fixtures and
/// real pages, not the full ~40-entry spec table.
const SVG_TAG_CASE_ADJUSTMENTS: &[(&str, &str)] = &[
    ("foreignobject", "foreignObject"),
    ("clippath", "clipPath"),
    ("lineargradient", "linearGradient"),
    ("radialgradient", "radialGradient"),
    ("textpath", "textPath"),
    ("viewbox", "viewBox"),
    ("glyphref", "glyphRef"),
];

pub(crate) fn adjust_svg_tag_name(lowercased: &str) -> String {
    for (from, to) in SVG_TAG_CASE_ADJUSTMENTS {
        if *from == lowercased {
            return (*to).to_string();
        }
    }
    lowercased.to_string()
}

/// MathML text-integration-point tags — HTML content is allowed as
/// children of these even while inside a MathML subtree per spec. Not
/// currently re-entered as ordinary HTML rules (see module docs) — no
/// production call site yet, so this is dead code by construction until a
/// follow-up pass wires in integration-point re-entry; kept (with its
/// test) as the ready-to-use predicate rather than deleted.
#[allow(dead_code)]
pub(crate) fn is_mathml_text_integration_point(tag: &str) -> bool {
    matches!(tag, "mi" | "mo" | "mn" | "ms" | "mtext")
}

/// SVG HTML-integration-point tags. Same not-yet-wired-in status as
/// [`is_mathml_text_integration_point`].
#[allow(dead_code)]
pub(crate) fn is_svg_html_integration_point(tag: &str) -> bool {
    matches!(tag, "foreignObject" | "desc" | "title")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_svg_tags_are_case_adjusted() {
        assert_eq!(adjust_svg_tag_name("foreignobject"), "foreignObject");
        assert_eq!(adjust_svg_tag_name("clippath"), "clipPath");
    }

    #[test]
    fn unknown_svg_tags_pass_through_unchanged() {
        assert_eq!(adjust_svg_tag_name("circle"), "circle");
        assert_eq!(adjust_svg_tag_name("path"), "path");
    }

    #[test]
    fn integration_point_predicates() {
        assert!(is_mathml_text_integration_point("mtext"));
        assert!(!is_mathml_text_integration_point("annotation-xml"));
        assert!(is_svg_html_integration_point("foreignObject"));
        assert!(!is_svg_html_integration_point("circle"));
    }
}
