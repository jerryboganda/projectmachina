//! Active formatting elements list (WHATWG HTML §13.2.4.3, design §2).
//! `attrs` is the original start-tag's attribute snapshot — needed because
//! the adoption agency algorithm's "create an element for the token" step
//! must recreate a formatting element with the *original* attributes, not
//! whatever the live DOM element's attributes currently are.

use machina_dom::{ElementHandle, Namespace};

/// `(afe_index, handle, attrs_snapshot, namespace)` — the pieces of a
/// matched formatting-element entry the adoption agency algorithm needs.
pub(crate) type FormattingElementMatch = (usize, ElementHandle, Vec<(String, String)>, Namespace);

#[derive(Debug, Clone)]
pub(crate) enum FormattingEntry {
    /// A scope marker (inserted when entering `<button>`, `<object>`,
    /// table cells, etc — reconstruction and the "clear up to the last
    /// marker" step never cross one).
    Marker,
    Element {
        handle: ElementHandle,
        tag: String,
        namespace: Namespace,
        attrs: Vec<(String, String)>,
    },
}

fn attrs_equal(a: &[(String, String)], b: &[(String, String)]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    // Order-sensitive per the Noah's Ark clause's own wording ("the same
    // number of attributes... and all those attributes have the same names
    // and values, but possibly in a different order" — WHATWG explicitly
    // allows reordering). Compare as multisets.
    let mut a_sorted = a.to_vec();
    let mut b_sorted = b.to_vec();
    a_sorted.sort();
    b_sorted.sort();
    a_sorted == b_sorted
}

#[derive(Debug, Default)]
pub(crate) struct ActiveFormattingElements(Vec<FormattingEntry>);

impl ActiveFormattingElements {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert_marker(&mut self) {
        self.0.push(FormattingEntry::Marker);
    }

    /// Pushes a new formatting element entry, applying the Noah's Ark
    /// clause first: if three (or more) entries since the last marker
    /// already match `tag`/`namespace`/`attrs`, the earliest of them is
    /// removed before the new one is pushed.
    pub fn push_with_noahs_ark(
        &mut self,
        handle: ElementHandle,
        tag: String,
        namespace: Namespace,
        attrs: Vec<(String, String)>,
    ) {
        let mut matches = Vec::new();
        for (i, entry) in self.0.iter().enumerate().rev() {
            match entry {
                FormattingEntry::Marker => break,
                FormattingEntry::Element {
                    tag: t,
                    namespace: n,
                    attrs: a,
                    ..
                } => {
                    if *t == tag && *n == namespace && attrs_equal(a, &attrs) {
                        matches.push(i);
                        if matches.len() >= 3 {
                            break;
                        }
                    }
                }
            }
        }
        // `matches` was collected walking backward from the end, so the
        // last element pushed onto it is the earliest (lowest-index) match
        // — remove that one. No `unwrap`/`expect`: `last()` returning
        // `None` here (it never should, since we only reach this branch
        // after `matches.len() >= 3`) simply skips the removal instead of
        // panicking on parser-input-reachable code.
        if matches.len() >= 3 {
            if let Some(&earliest) = matches.last() {
                self.0.remove(earliest);
            }
        }
        self.0.push(FormattingEntry::Element {
            handle,
            tag,
            namespace,
            attrs,
        });
    }

    pub fn clear_to_last_marker(&mut self) {
        while let Some(entry) = self.0.pop() {
            if matches!(entry, FormattingEntry::Marker) {
                break;
            }
        }
    }

    pub fn remove_element(&mut self, handle: ElementHandle) {
        self.0.retain(
            |entry| !matches!(entry, FormattingEntry::Element { handle: h, .. } if *h == handle),
        );
    }

    pub fn position_of(&self, handle: ElementHandle) -> Option<usize> {
        self.0.iter().position(
            |entry| matches!(entry, FormattingEntry::Element { handle: h, .. } if *h == handle),
        )
    }

    pub fn entry_at(&self, index: usize) -> Option<&FormattingEntry> {
        self.0.get(index)
    }

    pub fn replace_at(&mut self, index: usize, entry: FormattingEntry) {
        if let Some(slot) = self.0.get_mut(index) {
            *slot = entry;
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<FormattingEntry> {
        if index < self.0.len() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    pub fn insert_at(&mut self, index: usize, entry: FormattingEntry) {
        let index = index.min(self.0.len());
        self.0.insert(index, entry);
    }

    /// The last (nearest-end) `Element` entry with tag `tag`, searching
    /// backward but stopping at (and not crossing) a `Marker` — used by
    /// the adoption agency algorithm's step 4.3.
    pub fn last_between_end_and_marker_with_tag(
        &self,
        tag: &str,
    ) -> Option<FormattingElementMatch> {
        for (i, entry) in self.0.iter().enumerate().rev() {
            match entry {
                FormattingEntry::Marker => return None,
                FormattingEntry::Element {
                    handle,
                    tag: t,
                    attrs,
                    namespace,
                } if t == tag => return Some((i, *handle, attrs.clone(), *namespace)),
                FormattingEntry::Element { .. } => {}
            }
        }
        None
    }

    /// Iterates every `Element` entry from the end backward until (and
    /// excluding) the last marker, or the start of the list if there is no
    /// marker — used by "reconstruct the active formatting elements".
    pub fn entries(&self) -> &[FormattingEntry] {
        &self.0
    }
}
