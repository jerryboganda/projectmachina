//! Shared test-only `WrapperObserver` that records every callback it
//! receives, for assertions in `tests/observer.rs` and `tests/teardown.rs`.

use std::sync::{Arc, Mutex};

use machina_dom::{DocumentId, NodeChange, NodeHandle, WrapperObserver};

#[derive(Clone, Default)]
pub struct RecordingObserver {
    pub changed: Arc<Mutex<Vec<(NodeHandle, NodeChange)>>>,
    pub freed: Arc<Mutex<Vec<NodeHandle>>>,
    pub teardown: Arc<Mutex<Vec<DocumentId>>>,
}

impl RecordingObserver {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WrapperObserver for RecordingObserver {
    fn on_node_changed(&self, handle: NodeHandle, change: NodeChange) {
        self.changed.lock().expect("lock").push((handle, change));
    }

    fn on_node_freed(&self, handle: NodeHandle) {
        self.freed.lock().expect("lock").push(handle);
    }

    fn on_document_teardown(&self, document: DocumentId) {
        self.teardown.lock().expect("lock").push(document);
    }
}
