//! One arena per document: `alloc`/`free`/`resolve` with free-list reuse.
//! `resolve`/`resolve_mut` are the only ways to reach a [`NodeData`], and
//! both always go through `Vec::get`/`Vec::get_mut` (never index syntax),
//! so a corrupted or forged index can only ever produce
//! [`DomError::StaleHandle`], never a panic.

use crate::error::DomError;
use crate::handle::{DocumentId, Generation, NodeHandle, NodeIndex};
use crate::node::NodeData;

struct Slot {
    generation: Generation,
    data: Option<NodeData>,
}

#[derive(Default)]
pub(crate) struct NodeArena {
    slots: Vec<Slot>,
    free_list: Vec<u32>,
    live_count: u64,
}

impl NodeArena {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&mut self, document: DocumentId, data: NodeData) -> NodeHandle {
        if let Some(index) = self.free_list.pop() {
            // Free-list entries always index a slot that exists (pushed by
            // `free` below) and currently holds no data.
            if let Some(slot) = self.slots.get_mut(index as usize) {
                slot.data = Some(data);
                self.live_count += 1;
                return NodeHandle {
                    document,
                    index: NodeIndex(index),
                    generation: slot.generation,
                };
            }
        }
        let index = self.slots.len() as u32;
        let generation = Generation(0);
        self.slots.push(Slot {
            generation,
            data: Some(data),
        });
        self.live_count += 1;
        NodeHandle {
            document,
            index: NodeIndex(index),
            generation,
        }
    }

    pub(crate) fn resolve(&self, handle: NodeHandle) -> Result<&NodeData, DomError> {
        let slot = self
            .slots
            .get(handle.index.0 as usize)
            .ok_or(DomError::StaleHandle)?;
        if slot.generation != handle.generation {
            return Err(DomError::StaleHandle);
        }
        slot.data.as_ref().ok_or(DomError::StaleHandle)
    }

    pub(crate) fn resolve_mut(&mut self, handle: NodeHandle) -> Result<&mut NodeData, DomError> {
        let slot = self
            .slots
            .get_mut(handle.index.0 as usize)
            .ok_or(DomError::StaleHandle)?;
        if slot.generation != handle.generation {
            return Err(DomError::StaleHandle);
        }
        slot.data.as_mut().ok_or(DomError::StaleHandle)
    }

    /// Frees the node named by `handle`, bumping its slot's generation so
    /// every other handle into it becomes permanently stale.
    pub(crate) fn free(&mut self, handle: NodeHandle) -> Result<NodeData, DomError> {
        let slot = self
            .slots
            .get_mut(handle.index.0 as usize)
            .ok_or(DomError::StaleHandle)?;
        if slot.generation != handle.generation {
            return Err(DomError::StaleHandle);
        }
        let data = slot.data.take().ok_or(DomError::StaleHandle)?;
        slot.generation = Generation(slot.generation.0.wrapping_add(1));
        self.free_list.push(handle.index.0);
        self.live_count -= 1;
        Ok(data)
    }

    pub(crate) fn live_count(&self) -> u64 {
        self.live_count
    }
}
