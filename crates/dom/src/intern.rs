//! Two-tier name interning: a static, compiled-in table for well-known
//! tag/attribute names (process-lifetime, holds no page content) and a
//! dynamic per-document table for custom tags/attributes, which is dropped
//! with the owning document (bounding adversarial custom-tag growth to that
//! document's own accounted memory).
//!
//! Attribute values and text data are never interned here (`Box<str>`,
//! largely unique, owned directly by the node).

use std::collections::HashMap;
use std::sync::OnceLock;

/// A small `Copy` reference to an interned name, resolved through either the
/// static table or the owning document's dynamic interner.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Atom {
    Static(u32),
    Dynamic(u32),
}

const STATIC_NAMES: &[&str] = &[
    "html",
    "head",
    "body",
    "title",
    "meta",
    "link",
    "script",
    "style",
    "div",
    "span",
    "p",
    "a",
    "img",
    "ul",
    "ol",
    "li",
    "table",
    "tr",
    "td",
    "th",
    "thead",
    "tbody",
    "form",
    "input",
    "button",
    "label",
    "select",
    "option",
    "textarea",
    "iframe",
    "canvas",
    "svg",
    "path",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "br",
    "hr",
    "template",
    "id",
    "class",
    "name",
    "href",
    "src",
    "type",
    "value",
    "data",
    "alt",
    "rel",
    "content",
    "style",
    "width",
    "height",
    "role",
    "disabled",
    "checked",
    "placeholder",
    "title",
];

static STATIC_INDEX: OnceLock<HashMap<&'static str, u32>> = OnceLock::new();

fn static_index() -> &'static HashMap<&'static str, u32> {
    STATIC_INDEX.get_or_init(|| {
        let mut map = HashMap::with_capacity(STATIC_NAMES.len());
        for (index, name) in STATIC_NAMES.iter().enumerate() {
            // First occurrence wins for any accidental duplicate in the
            // table above; both entries still resolve to the same text.
            map.entry(*name).or_insert(index as u32);
        }
        map
    })
}

fn intern_static(name: &str) -> Option<Atom> {
    static_index().get(name).map(|index| Atom::Static(*index))
}

fn resolve_static(index: u32) -> &'static str {
    STATIC_NAMES.get(index as usize).copied().unwrap_or("")
}

/// Per-document dynamic interner for names not present in the static table.
#[derive(Debug, Default)]
pub(crate) struct StringInterner {
    names: Vec<Box<str>>,
    index: HashMap<Box<str>, u32>,
}

impl StringInterner {
    /// Interns `name`, inserting into the dynamic table if it is not a
    /// well-known static name and has not been seen before in this document.
    pub(crate) fn intern(&mut self, name: &str) -> Atom {
        if let Some(atom) = intern_static(name) {
            return atom;
        }
        if let Some(index) = self.index.get(name) {
            return Atom::Dynamic(*index);
        }
        let index = self.names.len() as u32;
        let boxed: Box<str> = name.into();
        self.index.insert(boxed.clone(), index);
        self.names.push(boxed);
        Atom::Dynamic(index)
    }

    /// Looks up `name` without interning it. Used by read-only accessors
    /// (`&self`) that must not mutate the interner just to fail a lookup.
    pub(crate) fn find(&self, name: &str) -> Option<Atom> {
        if let Some(atom) = intern_static(name) {
            return Some(atom);
        }
        self.index.get(name).map(|index| Atom::Dynamic(*index))
    }

    pub(crate) fn resolve(&self, atom: Atom) -> &str {
        match atom {
            Atom::Static(index) => resolve_static(index),
            Atom::Dynamic(index) => self
                .names
                .get(index as usize)
                .map(|value| value.as_ref())
                .unwrap_or(""),
        }
    }

    pub(crate) fn bytes_estimate(&self) -> u64 {
        self.names.iter().map(|name| name.len() as u64).sum()
    }
}
