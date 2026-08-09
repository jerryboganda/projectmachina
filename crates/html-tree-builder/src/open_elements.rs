//! Stack of open elements (WHATWG HTML §13.2.4.2), built directly on
//! `machina_dom::ElementHandle` (design §2). No parallel node-identity
//! type: each entry caches the tag/namespace captured at push time from
//! the creating token, so scope/tag comparisons during hot recovery
//! algorithms never round-trip through the DOM.

use machina_dom::{ElementHandle, Namespace};

#[derive(Debug, Clone)]
pub(crate) struct OpenElementEntry {
    pub handle: ElementHandle,
    pub tag: String,
    pub namespace: Namespace,
}

/// Tags in the WHATWG "implied end tags" list (§13.2.4.2). Popped
/// automatically by several algorithms (closing `<p>`, table cell/row
/// boundaries, ...) without a matching explicit end tag.
const IMPLIED_END_TAGS: &[&str] = &[
    "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
];

/// Extra tags implied-end-tags-"thoroughly" also pops (table-structure
/// boundaries).
const IMPLIED_END_TAGS_THOROUGHLY_EXTRA: &[&str] = &["tbody", "td", "tfoot", "th", "thead", "tr"];

/// The default-scope "stopper" tag set (§13.2.4.2's "has an element in
/// scope"). Used, with small additions, by every other scope-kind check.
fn is_default_scope_stopper(tag: &str, namespace: Namespace) -> bool {
    match namespace {
        Namespace::Html => matches!(
            tag,
            "applet"
                | "caption"
                | "html"
                | "table"
                | "td"
                | "th"
                | "marquee"
                | "object"
                | "template"
        ),
        Namespace::MathMl => matches!(tag, "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml"),
        Namespace::Svg => matches!(tag, "foreignObject" | "desc" | "title"),
    }
}

#[derive(Debug, Default)]
pub(crate) struct OpenElementsStack(Vec<OpenElementEntry>);

impl OpenElementsStack {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, entry: OpenElementEntry) {
        self.0.push(entry);
    }

    pub fn pop(&mut self) -> Option<OpenElementEntry> {
        self.0.pop()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Not currently called by any driving path (the stack always has at
    /// least the `<html>` element once parsing starts), but kept alongside
    /// `len` per clippy's `len_without_is_empty` lint and as a natural,
    /// cheap invariant check for future callers/tests.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn current(&self) -> Option<&OpenElementEntry> {
        self.0.last()
    }

    pub fn current_handle(&self) -> Option<ElementHandle> {
        self.0.last().map(|e| e.handle)
    }

    /// The bottommost (first-pushed) entry: the `<html>` element, once
    /// pushed. `None` before `BeforeHtml` inserts it.
    pub fn html_element(&self) -> Option<ElementHandle> {
        self.0.first().map(|e| e.handle)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &OpenElementEntry> {
        self.0.iter()
    }

    pub fn contains_html_tag(&self, tag: &str) -> bool {
        self.0
            .iter()
            .any(|e| e.namespace == Namespace::Html && e.tag == tag)
    }

    pub fn contains_handle(&self, handle: ElementHandle) -> bool {
        self.0.iter().any(|e| e.handle == handle)
    }

    pub fn index_of_handle(&self, handle: ElementHandle) -> Option<usize> {
        self.0.iter().position(|e| e.handle == handle)
    }

    pub fn entry_at(&self, index: usize) -> Option<&OpenElementEntry> {
        self.0.get(index)
    }

    /// Replaces the entry at `index` in place (adoption agency step "replace
    /// the entry for `node`").
    pub fn replace_at(&mut self, index: usize, entry: OpenElementEntry) {
        if let Some(slot) = self.0.get_mut(index) {
            *slot = entry;
        }
    }

    /// Removes the entry at `index` without disturbing the order of the
    /// rest of the stack (adoption agency needs this — it is not always a
    /// plain top-of-stack pop).
    pub fn remove_at(&mut self, index: usize) -> Option<OpenElementEntry> {
        if index < self.0.len() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    /// Inserts `entry` at `index`, shifting later entries up (adoption
    /// agency's "insert node into the stack of open elements immediately
    /// below X").
    pub fn insert_at(&mut self, index: usize, entry: OpenElementEntry) {
        let index = index.min(self.0.len());
        self.0.insert(index, entry);
    }

    /// Pops the stack down to and including the first (topmost) entry with
    /// tag name `tag` in the HTML namespace. No-op if `tag` is not on the
    /// stack.
    pub fn pop_until_html_tag(&mut self, tag: &str) {
        if !self.contains_html_tag(tag) {
            return;
        }
        while let Some(entry) = self.0.pop() {
            if entry.namespace == Namespace::Html && entry.tag == tag {
                break;
            }
        }
    }

    /// Pops the stack down to and including `handle`. No-op if `handle` is
    /// not on the stack.
    pub fn pop_until_handle(&mut self, handle: ElementHandle) {
        if !self.contains_handle(handle) {
            return;
        }
        while let Some(entry) = self.0.pop() {
            if entry.handle == handle {
                break;
            }
        }
    }

    pub fn pop_implied_end_tags(&mut self, except: Option<&str>, thoroughly: bool) {
        while let Some(top) = self.0.last() {
            if top.namespace != Namespace::Html {
                break;
            }
            if Some(top.tag.as_str()) == except {
                break;
            }
            let implied = IMPLIED_END_TAGS.contains(&top.tag.as_str())
                || (thoroughly && IMPLIED_END_TAGS_THOROUGHLY_EXTRA.contains(&top.tag.as_str()));
            if implied {
                self.0.pop();
            } else {
                break;
            }
        }
    }

    fn has_in_scope_with(&self, target: &str, stop: impl Fn(&str, Namespace) -> bool) -> bool {
        for entry in self.0.iter().rev() {
            if entry.namespace == Namespace::Html && entry.tag == target {
                return true;
            }
            if stop(&entry.tag, entry.namespace) {
                return false;
            }
        }
        false
    }

    pub fn has_in_scope(&self, target: &str) -> bool {
        self.has_in_scope_with(target, is_default_scope_stopper)
    }

    pub fn has_in_list_item_scope(&self, target: &str) -> bool {
        self.has_in_scope_with(target, |tag, ns| {
            is_default_scope_stopper(tag, ns)
                || (ns == Namespace::Html && matches!(tag, "ol" | "ul"))
        })
    }

    pub fn has_in_button_scope(&self, target: &str) -> bool {
        self.has_in_scope_with(target, |tag, ns| {
            is_default_scope_stopper(tag, ns) || (ns == Namespace::Html && tag == "button")
        })
    }

    pub fn has_in_table_scope(&self, target: &str) -> bool {
        self.has_in_scope_with(target, |tag, ns| {
            ns == Namespace::Html && matches!(tag, "html" | "table" | "template")
        })
    }

    pub fn has_in_select_scope(&self, target: &str) -> bool {
        self.has_in_scope_with(target, |tag, ns| {
            !(ns == Namespace::Html && matches!(tag, "optgroup" | "option"))
        })
    }

    /// `true` if any entry currently on the stack has HTML tag name `tag`
    /// (used by a handful of "if the stack of open elements has a `foo`
    /// element in it" checks that are not scope-bounded).
    pub fn has_html_tag_anywhere(&self, tag: &str) -> bool {
        self.contains_html_tag(tag)
    }
}
