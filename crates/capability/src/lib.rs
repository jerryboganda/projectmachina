//! Capability snapshots shared by native and Chromium engine adapters.
//!
//! A capability is eligible only when the snapshot explicitly reports it.
//! Missing, disabled, or unsupported capabilities never become silent success.

use std::collections::BTreeMap;

use machina_command_model::{CapabilityStatus, EngineKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilitySnapshot {
    pub engine: EngineKind,
    pub engine_build: String,
    capabilities: BTreeMap<String, CapabilityStatus>,
}

impl CapabilitySnapshot {
    pub fn new(engine: EngineKind, engine_build: impl Into<String>) -> Self {
        Self {
            engine,
            engine_build: engine_build.into(),
            capabilities: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        capability_id: impl Into<String>,
        status: CapabilityStatus,
    ) -> Option<CapabilityStatus> {
        self.capabilities.insert(capability_id.into(), status)
    }

    pub fn status(&self, capability_id: &str) -> Option<CapabilityStatus> {
        self.capabilities.get(capability_id).copied()
    }

    pub fn supports(&self, capability_id: &str) -> bool {
        matches!(
            (self.engine, self.status(capability_id)),
            (EngineKind::Native, Some(CapabilityStatus::Native))
                | (EngineKind::Native, Some(CapabilityStatus::NativeLimited))
                | (EngineKind::Native, Some(CapabilityStatus::Hybrid))
                | (EngineKind::Chromium, Some(CapabilityStatus::Chromium))
                | (EngineKind::Chromium, Some(CapabilityStatus::Hybrid))
        )
    }

    pub fn supports_all<'a, I>(&self, capability_ids: I) -> bool
    where
        I: IntoIterator<Item = &'a String>,
    {
        capability_ids
            .into_iter()
            .all(|capability_id| self.supports(capability_id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, CapabilityStatus)> {
        self.capabilities
            .iter()
            .map(|(id, status)| (id.as_str(), *status))
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilitySnapshot, CapabilityStatus, EngineKind};

    #[test]
    fn differentiates_native_and_chromium_support() {
        let mut native = CapabilitySnapshot::new(EngineKind::Native, "native-test");
        native.register("dom.query.v1", CapabilityStatus::Native);
        native.register("visual.screenshot.v1", CapabilityStatus::Chromium);
        assert!(native.supports("dom.query.v1"));
        assert!(!native.supports("visual.screenshot.v1"));

        let mut chromium = CapabilitySnapshot::new(EngineKind::Chromium, "chromium-test");
        chromium.register("visual.screenshot.v1", CapabilityStatus::Chromium);
        assert!(chromium.supports("visual.screenshot.v1"));
    }

    #[test]
    fn disabled_and_missing_capabilities_are_not_eligible() {
        let mut snapshot = CapabilitySnapshot::new(EngineKind::Native, "native-test");
        snapshot.register("dom.query.v1", CapabilityStatus::DisabledByPolicy);
        assert!(!snapshot.supports("dom.query.v1"));
        assert!(!snapshot.supports("missing.v1"));
    }
}
