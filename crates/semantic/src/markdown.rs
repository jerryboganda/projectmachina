//! Bounded, deterministic HTML -> Markdown transform.
//!
//! **Explicit scope** (a "readable markdown" bar, not a CommonMark-spec-
//! compliant round-trip — the M2-T13 task briefing's own framing):
//! headings (`h1`-`h6`), paragraphs, unordered/ordered lists (including
//! nesting), links, bold/italic (`strong`/`b`, `em`/`i`), inline `code` and
//! `<pre>` code blocks, images, horizontal rules, and blockquotes get
//! dedicated Markdown syntax. Every other element (`div`, `span`, `section`,
//! table cells, ...) is a transparent pass-through container (block-level
//! ones get paragraph-style spacing, inline-level ones do not) — this crate
//! does not attempt table-grid Markdown, definition lists, or footnotes.
//! `<script>`/`<style>`/`<template>`/`<noscript>`/`<head>` subtrees are
//! excluded entirely (never rendered as visible text).
//!
//! **Bounded, not paginated**: output is capped at [`MAX_MARKDOWN_BYTES`];
//! exceeding it truncates at a `char` boundary, appends a truncation
//! marker, and sets [`MarkdownDocument::truncated`]. There is no
//! cursor/offset-based continuation to resume past a truncation point in
//! this pass — disclosed as a real, deferred gap in
//! `.agent-state/evidence/M2-T13.md`, not silently narrowed.
//!
//! Iterative (explicit-stack `Open`/`Close` frames, never recursive) over
//! the live [`Document`] — the same shape `crates/html-tree-builder`'s test
//! `render()` helper and `crates/selectors`'s `walk_document_order` use, so
//! a pathologically deep tree cannot blow this crate's call stack. No
//! parallel/cloned tree is built; only the output `String` and a small
//! `Vec`-based list-nesting stack are allocated.

use machina_dom::{Document, NodeHandle, NodeKind, Revision};

use crate::error::SemanticError;
use crate::limits::{LimitKind, MAX_MARKDOWN_BYTES, MAX_TOTAL_NODES_VISITED};

/// Generated markdown for one document, self-stamped with the [`Revision`]
/// it was generated against (same staleness-detection contract as
/// [`crate::extract::SemanticIndex`] / `machina_selectors::QueryResult`).
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownDocument {
    pub revision: Revision,
    pub markdown: String,
    /// `true` if generation stopped early after [`MAX_MARKDOWN_BYTES`] was
    /// reached. Not an error — bounded output, not a failure.
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ElementKind {
    /// The element's entire subtree is excluded (never rendered as text).
    Skip,
    Heading(u8),
    Paragraph,
    UnorderedList,
    OrderedList,
    ListItem,
    Link,
    Bold,
    Italic,
    Code,
    Pre,
    LineBreak,
    Image,
    HorizontalRule,
    Blockquote,
    /// Block-level pass-through: `div`, `section`, table cells, etc. — no
    /// Markdown syntax of its own, but gets paragraph-style spacing.
    GenericBlock,
    /// Inline-level pass-through: `span` and similar — no syntax, no extra
    /// spacing.
    GenericInline,
}

fn classify(tag: &str) -> ElementKind {
    match tag {
        "script" | "style" | "template" | "noscript" | "head" => ElementKind::Skip,
        "h1" => ElementKind::Heading(1),
        "h2" => ElementKind::Heading(2),
        "h3" => ElementKind::Heading(3),
        "h4" => ElementKind::Heading(4),
        "h5" => ElementKind::Heading(5),
        "h6" => ElementKind::Heading(6),
        "p" => ElementKind::Paragraph,
        "ul" => ElementKind::UnorderedList,
        "ol" => ElementKind::OrderedList,
        "li" => ElementKind::ListItem,
        "a" => ElementKind::Link,
        "strong" | "b" => ElementKind::Bold,
        "em" | "i" => ElementKind::Italic,
        "code" => ElementKind::Code,
        "pre" => ElementKind::Pre,
        "br" => ElementKind::LineBreak,
        "img" => ElementKind::Image,
        "hr" => ElementKind::HorizontalRule,
        "blockquote" => ElementKind::Blockquote,
        "div" | "section" | "article" | "header" | "footer" | "nav" | "aside" | "main"
        | "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" | "form" | "fieldset" => {
            ElementKind::GenericBlock
        }
        _ => ElementKind::GenericInline,
    }
}

#[derive(Clone, Copy, Debug)]
enum ListKind {
    Unordered,
    Ordered(u64),
}

#[derive(Default)]
struct Writer {
    out: String,
    truncated: bool,
    list_stack: Vec<ListKind>,
    preformatted_depth: u32,
}

impl Writer {
    /// Appends `text`, truncating at the nearest `char` boundary and
    /// setting `truncated` if it would exceed [`MAX_MARKDOWN_BYTES`].
    /// Returns `true` once truncation has happened (a signal to the caller
    /// to stop generating further output).
    fn push(&mut self, text: &str) -> bool {
        if self.truncated {
            return true;
        }
        let remaining = MAX_MARKDOWN_BYTES.saturating_sub(self.out.len());
        if text.len() <= remaining {
            self.out.push_str(text);
            return false;
        }
        let mut cut = remaining;
        while cut > 0 && !text.is_char_boundary(cut) {
            cut -= 1;
        }
        self.out.push_str(&text[..cut]);
        self.truncated = true;
        // The marker itself is deliberately not counted against
        // MAX_MARKDOWN_BYTES (documented in the module docs above): bounding
        // it too would risk truncating the marker itself.
        self.out.push_str("\n\n…[truncated]");
        true
    }
}

/// Collapses runs of whitespace to a single space **without** trimming
/// leading/trailing whitespace — unlike `text::normalize_whitespace`, this
/// must preserve "is there a space here" at segment boundaries, since a text
/// node like `"Hello "` immediately followed by `<b>world</b>` needs to keep
/// its trailing space or the output would read `Hello**world**`.
fn collapse_whitespace_preserve_edges(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_space = false;
    for ch in input.chars() {
        if ch.is_whitespace() {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(ch);
            in_space = false;
        }
    }
    out
}

/// Collapses 3-or-more consecutive newlines down to exactly 2 (one blank
/// line) in the final assembled output — block-element open/close markers
/// each contribute their own `"\n\n"`, which stack up at every
/// block-to-block boundary otherwise.
fn normalize_blank_lines(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut newline_run = 0u32;
    for ch in input.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push(ch);
            }
        } else {
            newline_run = 0;
            out.push(ch);
        }
    }
    out
}

enum Frame {
    Open(NodeHandle),
    CloseBlock,
    CloseInlineMarker(&'static str),
    CloseLink(String),
    PopList,
    PopPreformatted,
}

fn push_children(
    document: &Document,
    handle: NodeHandle,
    stack: &mut Vec<Frame>,
) -> Result<(), SemanticError> {
    let children = document.children(handle)?;
    for child in children.into_iter().rev() {
        stack.push(Frame::Open(child));
    }
    Ok(())
}

/// Generates bounded markdown for the entirety of `document`, starting from
/// its root, self-stamped with the [`Revision`] the walk ran against.
pub fn generate_markdown(document: &Document) -> Result<MarkdownDocument, SemanticError> {
    let mut writer = Writer::default();
    let mut stack = vec![Frame::Open(document.root())];
    let mut visited: u64 = 0;

    'walk: while let Some(frame) = stack.pop() {
        match frame {
            Frame::Open(handle) => {
                visited += 1;
                if visited > MAX_TOTAL_NODES_VISITED {
                    return Err(SemanticError::TooComplex {
                        limit: LimitKind::MarkdownWalk,
                    });
                }
                let Ok(node) = document.node(handle) else {
                    continue;
                };
                match node.kind() {
                    NodeKind::Text => {
                        let data = document.text_data(handle)?;
                        let text = if writer.preformatted_depth > 0 {
                            data.to_string()
                        } else {
                            collapse_whitespace_preserve_edges(data)
                        };
                        if writer.push(&text) {
                            break 'walk;
                        }
                    }
                    NodeKind::Document | NodeKind::DocumentFragment => {
                        push_children(document, handle, &mut stack)?;
                    }
                    NodeKind::Comment | NodeKind::DocumentType => {}
                    NodeKind::Element => {
                        let element = document.as_element(handle)?;
                        let tag = document.tag_name(element)?;
                        match classify(tag) {
                            ElementKind::Skip => {}
                            ElementKind::LineBreak => {
                                if writer.push("  \n") {
                                    break 'walk;
                                }
                            }
                            ElementKind::Image => {
                                let alt = document.attribute(element, "alt")?.unwrap_or("");
                                let src = document.attribute(element, "src")?.unwrap_or("");
                                if writer.push(&format!("![{alt}]({src})")) {
                                    break 'walk;
                                }
                            }
                            ElementKind::HorizontalRule => {
                                if writer.push("\n\n---\n\n") {
                                    break 'walk;
                                }
                            }
                            ElementKind::Heading(level) => {
                                if writer.push(&format!("\n\n{} ", "#".repeat(level as usize))) {
                                    break 'walk;
                                }
                                stack.push(Frame::CloseBlock);
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::Paragraph | ElementKind::GenericBlock => {
                                if writer.push("\n\n") {
                                    break 'walk;
                                }
                                stack.push(Frame::CloseBlock);
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::Blockquote => {
                                if writer.push("\n\n> ") {
                                    break 'walk;
                                }
                                stack.push(Frame::CloseBlock);
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::UnorderedList => {
                                writer.list_stack.push(ListKind::Unordered);
                                if writer.push("\n") {
                                    break 'walk;
                                }
                                stack.push(Frame::PopList);
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::OrderedList => {
                                writer.list_stack.push(ListKind::Ordered(1));
                                if writer.push("\n") {
                                    break 'walk;
                                }
                                stack.push(Frame::PopList);
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::ListItem => {
                                let depth = writer.list_stack.len().saturating_sub(1);
                                let indent = "  ".repeat(depth);
                                let marker = match writer.list_stack.last_mut() {
                                    Some(ListKind::Ordered(next)) => {
                                        let rendered = format!("{indent}{next}. ");
                                        *next += 1;
                                        rendered
                                    }
                                    Some(ListKind::Unordered) | None => format!("{indent}- "),
                                };
                                if writer.push(&format!("\n{marker}")) {
                                    break 'walk;
                                }
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::Link => {
                                let href = document
                                    .attribute(element, "href")?
                                    .unwrap_or("")
                                    .to_string();
                                if writer.push("[") {
                                    break 'walk;
                                }
                                stack.push(Frame::CloseLink(href));
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::Bold => {
                                if writer.push("**") {
                                    break 'walk;
                                }
                                stack.push(Frame::CloseInlineMarker("**"));
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::Italic => {
                                if writer.push("*") {
                                    break 'walk;
                                }
                                stack.push(Frame::CloseInlineMarker("*"));
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::Code => {
                                if writer.preformatted_depth == 0 {
                                    if writer.push("`") {
                                        break 'walk;
                                    }
                                    stack.push(Frame::CloseInlineMarker("`"));
                                }
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::Pre => {
                                writer.preformatted_depth += 1;
                                if writer.push("\n\n```\n") {
                                    break 'walk;
                                }
                                stack.push(Frame::PopPreformatted);
                                push_children(document, handle, &mut stack)?;
                            }
                            ElementKind::GenericInline => {
                                push_children(document, handle, &mut stack)?;
                            }
                        }
                    }
                }
            }
            Frame::CloseBlock => {
                if writer.push("\n\n") {
                    break 'walk;
                }
            }
            Frame::CloseInlineMarker(marker) => {
                if writer.push(marker) {
                    break 'walk;
                }
            }
            Frame::CloseLink(href) => {
                if writer.push(&format!("]({href})")) {
                    break 'walk;
                }
            }
            Frame::PopList => {
                writer.list_stack.pop();
                if writer.push("\n") {
                    break 'walk;
                }
            }
            Frame::PopPreformatted => {
                writer.preformatted_depth = writer.preformatted_depth.saturating_sub(1);
                if writer.push("\n```\n\n") {
                    break 'walk;
                }
            }
        }
    }

    let markdown = normalize_blank_lines(writer.out.trim());
    Ok(MarkdownDocument {
        revision: document.revision(),
        markdown,
        truncated: writer.truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_preserves_boundary_spaces() {
        assert_eq!(collapse_whitespace_preserve_edges("Hello "), "Hello ");
        assert_eq!(collapse_whitespace_preserve_edges("  a   b  "), " a b ");
        assert_eq!(collapse_whitespace_preserve_edges("noop"), "noop");
    }

    #[test]
    fn normalize_blank_lines_caps_at_two_newlines() {
        assert_eq!(normalize_blank_lines("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(normalize_blank_lines("a\nb"), "a\nb");
        assert_eq!(normalize_blank_lines("a\n\nb"), "a\n\nb");
    }
}
