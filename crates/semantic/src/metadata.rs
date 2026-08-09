//! Metadata/schema extraction: `<title>`, `<html lang>`, and `<meta>` tag
//! key/value pairs. Deliberately scoped small — see module docs below for
//! exactly what is and is not extracted.
//!
//! **In scope**: document title (first `<title>` element's bounded text
//! content), the root `<html>` element's `lang` attribute, and every
//! `<meta>` element expressed as one key/value pair (`charset`, `name`
//! +`content`, `property`+`content` for Open Graph-style tags, or
//! `http-equiv`+`content`).
//!
//! **Out of scope, disclosed**: JSON-LD / `<script type="application/ld+json">`
//! structured-data parsing, microdata (`itemprop`/`itemscope`), RDFa, and
//! `<link>` tag extraction (canonical URL, favicon, ...). None of these are
//! silently mis-extracted — they simply are not walked by this pass. A
//! real, disclosed gap for a later task, not a corner cut in this one (see
//! `.agent-state/evidence/M2-T13.md`).

use machina_dom::{Document, Revision};

use crate::error::SemanticError;
use crate::limits::{MAX_METADATA_VALUE_CHARS, MAX_SEMANTIC_ITEMS};
use crate::text::{collect_text_content, normalize_whitespace};
use crate::walk::walk_document_order;

/// Extracted document metadata, self-stamped with the [`Revision`] it was
/// computed against (same staleness-detection contract as
/// [`crate::extract::SemanticIndex`] / [`crate::markdown::MarkdownDocument`]).
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentMetadata {
    pub revision: Revision,
    pub title: Option<String>,
    pub lang: Option<String>,
    /// `(key, value)` pairs in document order. See module docs for exactly
    /// how each `<meta>` shape maps to a key.
    pub meta: Vec<(String, String)>,
    /// `true` if `meta` stopped accumulating entries after
    /// [`MAX_SEMANTIC_ITEMS`] (bounded output, not an error).
    pub truncated: bool,
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn meta_key_value(
    document: &Document,
    element: machina_dom::ElementHandle,
) -> Result<Option<(String, String)>, SemanticError> {
    if let Some(charset) = document.attribute(element, "charset")? {
        return Ok(Some((
            "charset".to_string(),
            truncate_chars(&normalize_whitespace(charset), MAX_METADATA_VALUE_CHARS),
        )));
    }
    let content = document.attribute(element, "content")?;
    let Some(content) = content else {
        return Ok(None);
    };
    let value = truncate_chars(&normalize_whitespace(content), MAX_METADATA_VALUE_CHARS);
    for attr in ["name", "property", "http-equiv"] {
        if let Some(key) = document.attribute(element, attr)? {
            let key = truncate_chars(&normalize_whitespace(key), MAX_METADATA_VALUE_CHARS);
            if !key.is_empty() {
                return Ok(Some((key, value)));
            }
        }
    }
    Ok(None)
}

/// Extracts [`DocumentMetadata`] for `document`, computed fresh against its
/// current [`Revision`].
pub fn extract_metadata(document: &Document) -> Result<DocumentMetadata, SemanticError> {
    let mut title = None;
    let mut lang = None;
    let mut meta = Vec::new();
    let mut truncated = false;

    walk_document_order(document, |handle| {
        let Ok(element) = document.as_element(handle) else {
            return Ok(());
        };
        let tag = document.tag_name(element)?;
        match tag {
            "html" if lang.is_none() => {
                if let Some(value) = document.attribute(element, "lang")? {
                    let normalized = normalize_whitespace(value);
                    if !normalized.is_empty() {
                        lang = Some(truncate_chars(&normalized, MAX_METADATA_VALUE_CHARS));
                    }
                }
            }
            "title" if title.is_none() => {
                let (text, _) = collect_text_content(document, handle, MAX_METADATA_VALUE_CHARS)?;
                let normalized = normalize_whitespace(&text);
                if !normalized.is_empty() {
                    title = Some(normalized);
                }
            }
            "meta" => {
                if let Some(pair) = meta_key_value(document, element)? {
                    if meta.len() < MAX_SEMANTIC_ITEMS {
                        meta.push(pair);
                    } else {
                        truncated = true;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })?;

    Ok(DocumentMetadata {
        revision: document.revision(),
        title,
        lang,
        meta,
        truncated,
    })
}
