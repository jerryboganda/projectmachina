//! Basic role and accessible-name derivation.
//!
//! **Explicit scope** (this is the "basic, native-HTML-semantics-only
//! subset" named in the M2-T13 task briefing, not a full ARIA computation
//! engine):
//!
//! - **Role**: an explicit `role="..."` attribute always wins (first
//!   whitespace-separated token, lowercased — this crate does not implement
//!   the full ARIA "role fallback list" of trying subsequent tokens when the
//!   first is unrecognized). Otherwise, a small, curated table of *implicit*
//!   HTML-AAM roles for the elements the task briefing names explicitly
//!   (interactive elements, headings, links, forms) plus a handful of common
//!   landmark/structural tags. Every other element has no derived role
//!   (`None`) — this is not a gap, most elements (`div`, `span`, `p`, ...)
//!   genuinely have no implicit ARIA role.
//! - **Accessible name**: `aria-label` first, then a small set of
//!   native-HTML name sources (`alt` for `img`, associated `<label>` for
//!   form controls, `placeholder` as a form-control fallback), then a
//!   bounded, whitespace-collapsed text-content fallback for everything
//!   else. **Not implemented**: `aria-labelledby` (ID-reference resolution
//!   across the tree), `aria-describedby`, `title`-attribute fallback,
//!   visibility/`aria-hidden`/`display:none` exclusion (this crate has no
//!   layout/CSS model — see the M2-T05/M2-T10 design docs' own non-goals),
//!   table `<caption>`, `<fieldset>`/`<legend>`, and the full W3C
//!   accname-1.2 precedence algorithm. Disclosed in
//!   `.agent-state/evidence/M2-T13.md`, not silently narrowed.

use machina_dom::{Document, ElementHandle};

use crate::error::SemanticError;
use crate::limits::{MAX_ACCESSIBLE_NAME_CHARS, MAX_ANCESTOR_LOOKUP_WALK};
use crate::text::{collect_text_content, normalize_whitespace};
use std::collections::HashMap;

/// Implicit-role table entry lookup keyed by lowercase tag name, for the
/// tags that do not need attribute-dependent logic (`input`/`select` need
/// their own branches below).
fn implicit_role_for_tag(tag: &str) -> Option<&'static str> {
    match tag {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some("heading"),
        "button" => Some("button"),
        "textarea" => Some("textbox"),
        "nav" => Some("navigation"),
        "main" => Some("main"),
        "form" => Some("form"),
        "img" => Some("img"),
        "ul" | "ol" => Some("list"),
        "li" => Some("listitem"),
        "table" => Some("table"),
        "summary" => Some("button"),
        _ => None,
    }
}

/// `Some(1..=6)` for `h1`..`h6`, `None` otherwise. Separate from the general
/// role table because heading level is extra structured data the ARIA
/// `aria-level` attribute would carry, not part of the role string itself.
pub(crate) fn heading_level(tag: &str) -> Option<u8> {
    match tag {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

/// Implicit role for `<input>`, which HTML-AAM maps by `type` attribute
/// rather than tag name alone. Curated to the common, high-frequency
/// automation-relevant `type` values named in the task briefing's
/// "interactive elements" deliverable; an input `type` not listed here
/// falls back to `"textbox"` (HTML-AAM's own default for `<input>` with no
/// recognized special role), except `hidden`, which has no role (never
/// user-interactive).
fn implicit_role_for_input(input_type: &str) -> Option<&'static str> {
    match input_type {
        "hidden" => None,
        "button" | "submit" | "reset" | "image" => Some("button"),
        "checkbox" => Some("checkbox"),
        "radio" => Some("radio"),
        "range" => Some("slider"),
        "search" => Some("searchbox"),
        _ => Some("textbox"),
    }
}

/// Roles this crate treats as "interactive" for
/// [`crate::extract::SemanticIndex::interactive_elements`]. **Scope note**:
/// this is role-driven, so a custom widget that carries `tabindex` but no
/// ARIA role (a common but non-conformant real-world pattern) is not
/// flagged interactive by this pass — disclosed as a real gap in
/// `.agent-state/evidence/M2-T13.md`, not silently handled.
pub(crate) const INTERACTIVE_ROLES: &[&str] = &[
    "button",
    "link",
    "checkbox",
    "radio",
    "combobox",
    "listbox",
    "slider",
    "textbox",
    "searchbox",
];

/// Derives the role for `element`, or `None` if it has no ARIA role by this
/// crate's rules (explicit `role="..."` attribute, else the curated
/// implicit-role table above). Reads only `element`'s own attributes/tag —
/// no tree walk.
pub(crate) fn derive_role(
    document: &Document,
    element: ElementHandle,
) -> Result<Option<String>, SemanticError> {
    if let Some(explicit) = document.attribute(element, "role")? {
        if let Some(token) = explicit.split_ascii_whitespace().next() {
            if !token.is_empty() {
                return Ok(Some(token.to_ascii_lowercase()));
            }
        }
    }

    let tag = document.tag_name(element)?;
    let role = match tag {
        "a" => document
            .attribute(element, "href")?
            .is_some()
            .then_some("link"),
        "input" => {
            let raw_type = document.attribute(element, "type")?.unwrap_or("");
            let lowered = raw_type.to_ascii_lowercase();
            let input_type = if lowered.is_empty() {
                "text"
            } else {
                lowered.as_str()
            };
            implicit_role_for_input(input_type)
        }
        "select" => {
            if document.attribute(element, "multiple")?.is_some() {
                Some("listbox")
            } else {
                Some("combobox")
            }
        }
        _ => implicit_role_for_tag(tag),
    };
    Ok(role.map(str::to_string))
}

/// Resolves a form control's accessible name via an explicit `<label
/// for="...">` (looked up in `for_map`, keyed by the `for` attribute's raw
/// value) or, failing that, a wrapping `<label>` ancestor (walked upward,
/// bounded by [`MAX_ANCESTOR_LOOKUP_WALK`]). Returns the label element's own
/// bounded text content, normalized.
fn label_derived_name(
    document: &Document,
    element: ElementHandle,
    for_map: &HashMap<String, ElementHandle>,
) -> Result<Option<String>, SemanticError> {
    if let Some(id) = document.attribute(element, "id")? {
        if let Some(&label) = for_map.get(id) {
            let (text, _) =
                collect_text_content(document, label.node_handle(), MAX_ACCESSIBLE_NAME_CHARS)?;
            let normalized = normalize_whitespace(&text);
            if !normalized.is_empty() {
                return Ok(Some(normalized));
            }
        }
    }

    // Wrapping-label fallback: `<label>Name <input></label>`.
    let mut current = document.node(element.node_handle())?.parent();
    let mut steps = 0;
    while let Some(handle) = current {
        steps += 1;
        if steps > MAX_ANCESTOR_LOOKUP_WALK {
            break;
        }
        let Ok(ancestor_element) = document.as_element(handle) else {
            current = document.node(handle)?.parent();
            continue;
        };
        if document.tag_name(ancestor_element)? == "label" {
            let (text, _) = collect_text_content(
                document,
                ancestor_element.node_handle(),
                MAX_ACCESSIBLE_NAME_CHARS,
            )?;
            let normalized = normalize_whitespace(&text);
            if !normalized.is_empty() {
                return Ok(Some(normalized));
            }
            break;
        }
        current = document.node(handle)?.parent();
    }
    Ok(None)
}

/// Derives the accessible name for `element` per this module's documented
/// scope. `for_map` (`for`-attribute value -> `<label>` element) comes from
/// the single label-indexing pass (`extract::build_label_map`) built once
/// per extraction call, not recomputed per element.
pub(crate) fn derive_accessible_name(
    document: &Document,
    element: ElementHandle,
    for_map: &HashMap<String, ElementHandle>,
) -> Result<Option<String>, SemanticError> {
    if let Some(value) = document.attribute(element, "aria-label")? {
        let normalized = normalize_whitespace(value);
        if !normalized.is_empty() {
            return Ok(Some(truncate_chars(&normalized, MAX_ACCESSIBLE_NAME_CHARS)));
        }
    }

    let tag = document.tag_name(element)?;

    if tag == "img" {
        return Ok(document.attribute(element, "alt")?.and_then(|alt| {
            let normalized = normalize_whitespace(alt);
            if normalized.is_empty() {
                None
            } else {
                Some(truncate_chars(&normalized, MAX_ACCESSIBLE_NAME_CHARS))
            }
        }));
    }

    if matches!(tag, "input" | "textarea" | "select") {
        if let Some(name) = label_derived_name(document, element, for_map)? {
            return Ok(Some(truncate_chars(&name, MAX_ACCESSIBLE_NAME_CHARS)));
        }
        if let Some(placeholder) = document.attribute(element, "placeholder")? {
            let normalized = normalize_whitespace(placeholder);
            if !normalized.is_empty() {
                return Ok(Some(truncate_chars(&normalized, MAX_ACCESSIBLE_NAME_CHARS)));
            }
        }
        return Ok(None);
    }

    // `<form>` deliberately does not fall through to the generic
    // text-content fallback below: a form's descendant text (every label,
    // every option) is not a meaningful "name" for the form itself, and
    // computing it would be both misleading and needlessly expensive for
    // large forms. Only an explicit `aria-label` (handled above) names a
    // form in this crate's scope.
    if tag == "form" {
        return Ok(None);
    }

    let (text, _) =
        collect_text_content(document, element.node_handle(), MAX_ACCESSIBLE_NAME_CHARS)?;
    let normalized = normalize_whitespace(&text);
    if normalized.is_empty() {
        Ok(None)
    } else {
        Ok(Some(truncate_chars(&normalized, MAX_ACCESSIBLE_NAME_CHARS)))
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_level_covers_h1_through_h6_only() {
        for (tag, expected) in [
            ("h1", Some(1)),
            ("h2", Some(2)),
            ("h3", Some(3)),
            ("h4", Some(4)),
            ("h5", Some(5)),
            ("h6", Some(6)),
            ("h7", None),
            ("div", None),
        ] {
            assert_eq!(heading_level(tag), expected, "tag={tag}");
        }
    }

    #[test]
    fn implicit_input_role_table_matches_documented_subset() {
        assert_eq!(implicit_role_for_input("hidden"), None);
        assert_eq!(implicit_role_for_input("submit"), Some("button"));
        assert_eq!(implicit_role_for_input("checkbox"), Some("checkbox"));
        assert_eq!(implicit_role_for_input("radio"), Some("radio"));
        assert_eq!(implicit_role_for_input("range"), Some("slider"));
        assert_eq!(implicit_role_for_input("search"), Some("searchbox"));
        assert_eq!(implicit_role_for_input("text"), Some("textbox"));
        assert_eq!(implicit_role_for_input("email"), Some("textbox"));
        assert_eq!(implicit_role_for_input("made-up-type"), Some("textbox"));
    }
}
