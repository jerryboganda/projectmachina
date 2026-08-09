//! Bounded, iterative (never recursive) text-content collection shared by
//! accessible-name derivation and `<title>`/metadata extraction. Reads
//! directly from the live [`Document`] — no parallel tree/string copy of
//! the subtree is ever built beyond the one `String` this function returns.

use machina_dom::{Document, NodeHandle, NodeKind};

use crate::error::SemanticError;
use crate::limits::MAX_TEXT_WALK_NODES;

/// Element tags whose entire subtree never contributes to a text-content
/// computation (script/style content is not visible text; `template`
/// content is inert markup, not rendered document content).
fn is_text_excluded_subtree(tag: &str) -> bool {
    matches!(tag, "script" | "style" | "template" | "noscript")
}

/// Collects the concatenated text of every `Text` node under `root` (`root`
/// itself included if it is a `Text` node), in document order, skipping the
/// subtree of any [`is_text_excluded_subtree`] element. Iterative
/// (explicit-stack, never recursive) so a pathologically deep tree cannot
/// blow this crate's call stack — mirrors `machina_dom`'s own
/// `clone_node`/`collect_children` iterative posture and
/// `machina_selectors`'s `walk_document_order`.
///
/// Bounded by `max_chars` (the returned string never exceeds this many
/// `char`s — checked, not merely byte-truncated, so multi-byte UTF-8 is
/// never split mid-codepoint) and by [`MAX_TEXT_WALK_NODES`] (a defensive
/// backstop on total nodes visited). Either bound being hit sets the
/// returned `truncated` flag; this is normal bounded-output behavior, never
/// a [`SemanticError`] — an oversized subtree under one element must not
/// fail the whole extraction, it should just produce a truncated name for
/// that one element.
pub(crate) fn collect_text_content(
    document: &Document,
    root: NodeHandle,
    max_chars: usize,
) -> Result<(String, bool), SemanticError> {
    let mut out = String::new();
    let mut truncated = false;
    let mut stack = vec![root];
    let mut visited: u64 = 0;

    'walk: while let Some(handle) = stack.pop() {
        visited += 1;
        if visited > MAX_TEXT_WALK_NODES {
            truncated = true;
            break;
        }
        let node = match document.node(handle) {
            Ok(node) => node,
            Err(_) => continue,
        };
        match node.kind() {
            NodeKind::Text => {
                let data = document.text_data(handle)?;
                for ch in data.chars() {
                    if out.chars().count() >= max_chars {
                        truncated = true;
                        break 'walk;
                    }
                    out.push(ch);
                }
            }
            NodeKind::Element => {
                let element = document.as_element(handle)?;
                let tag = document.tag_name(element)?;
                if is_text_excluded_subtree(tag) {
                    continue;
                }
                let children = document.children(handle)?;
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
            }
            _ => {}
        }
    }

    Ok((out, truncated))
}

/// Collapses every run of ASCII/Unicode whitespace to a single space and
/// trims the ends — the same "flatten to one line for a name/label" bound
/// every accname-style computation performs. Not full Unicode
/// line-breaking/segmentation, just the common `\s+` -> `" "` fold.
pub(crate) fn normalize_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_was_space = true; // trims leading whitespace for free
    for ch in input.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
            }
            last_was_space = true;
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_and_trims() {
        assert_eq!(normalize_whitespace("  a\n\tb   c  "), "a b c");
        assert_eq!(normalize_whitespace(""), "");
        assert_eq!(normalize_whitespace("   "), "");
        assert_eq!(normalize_whitespace("solo"), "solo");
    }
}
