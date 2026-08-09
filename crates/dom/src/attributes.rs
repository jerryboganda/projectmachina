//! `AttributeMap`: an ordered, linear-scan attribute collection matching
//! `NamedNodeMap` semantics (insertion order preserved, small counts
//! expected per element) rather than a `HashMap`.

use crate::intern::Atom;

#[derive(Debug, Default, Clone)]
pub(crate) struct AttributeMap {
    entries: Vec<(Atom, Box<str>)>,
}

impl AttributeMap {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get(&self, atom: Atom) -> Option<&str> {
        self.entries
            .iter()
            .find(|(entry_atom, _)| *entry_atom == atom)
            .map(|(_, value)| value.as_ref())
    }

    /// Sets `atom` to `value`, returning the byte length of the previous
    /// value (if any) so the caller can maintain an incremental byte
    /// estimate.
    pub(crate) fn set(&mut self, atom: Atom, value: Box<str>) -> Option<u64> {
        if let Some(entry) = self.entries.iter_mut().find(|(a, _)| *a == atom) {
            let previous_len = entry.1.len() as u64;
            entry.1 = value;
            Some(previous_len)
        } else {
            self.entries.push((atom, value));
            None
        }
    }

    /// Removes `atom`, returning the byte length of the removed value (if
    /// it was present).
    pub(crate) fn remove(&mut self, atom: Atom) -> Option<u64> {
        if let Some(position) = self.entries.iter().position(|(a, _)| *a == atom) {
            let (_, value) = self.entries.remove(position);
            Some(value.len() as u64)
        } else {
            None
        }
    }

    pub(crate) fn bytes_estimate(&self) -> u64 {
        self.entries
            .iter()
            .map(|(_, value)| value.len() as u64)
            .sum()
    }

    /// Resolves every entry's atom to its name via `doc`'s interner and
    /// clones the value, for carrying an element's attributes into an
    /// owned, document-independent snapshot (`clone_node`/`adopt_node`).
    pub(crate) fn pairs_for_clone(
        &self,
        doc: &crate::document::Document,
    ) -> Vec<(String, Box<str>)> {
        self.entries
            .iter()
            .map(|(atom, value)| (doc.interner.resolve(*atom).to_string(), value.clone()))
            .collect()
    }
}
