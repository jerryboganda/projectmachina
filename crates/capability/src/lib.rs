//! Capability snapshots shared by native and Chromium engine adapters.
//!
//! A capability is eligible only when the snapshot explicitly reports it.
//! Missing, disabled, or unsupported capabilities never become silent success.
//! Snapshots carry a schema version and expose the same response consumed by
//! routing and capability APIs.

use std::collections::BTreeMap;

use machina_command_model::{CapabilityStatus, EngineKind};
use serde::{Deserialize, Serialize};

pub const CAPABILITY_SNAPSHOT_VERSION: &str = "capability.v0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRecord {
    pub status: CapabilityStatus,
    pub policy_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityEntry {
    pub id: String,
    pub status: CapabilityStatus,
    pub policy_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityResponse {
    pub engine: EngineKind,
    pub engine_build: String,
    pub snapshot_version: String,
    pub capabilities: Vec<CapabilityEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilitySnapshot {
    pub engine: EngineKind,
    pub engine_build: String,
    pub snapshot_version: String,
    capabilities: BTreeMap<String, CapabilityRecord>,
}

impl CapabilitySnapshot {
    pub fn new(engine: EngineKind, engine_build: impl Into<String>) -> Self {
        Self::versioned(engine, engine_build, CAPABILITY_SNAPSHOT_VERSION)
    }

    pub fn versioned(
        engine: EngineKind,
        engine_build: impl Into<String>,
        snapshot_version: impl Into<String>,
    ) -> Self {
        Self {
            engine,
            engine_build: engine_build.into(),
            snapshot_version: snapshot_version.into(),
            capabilities: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        capability_id: impl Into<String>,
        status: CapabilityStatus,
    ) -> Option<CapabilityStatus> {
        let policy_enabled = status != CapabilityStatus::DisabledByPolicy;
        self.register_with_policy(capability_id, status, policy_enabled)
    }

    pub fn register_with_policy(
        &mut self,
        capability_id: impl Into<String>,
        status: CapabilityStatus,
        policy_enabled: bool,
    ) -> Option<CapabilityStatus> {
        let policy_enabled = policy_enabled && status != CapabilityStatus::DisabledByPolicy;
        self.capabilities
            .insert(
                capability_id.into(),
                CapabilityRecord {
                    status,
                    policy_enabled,
                },
            )
            .map(|record| Self::effective_status(&record))
    }

    pub fn status(&self, capability_id: &str) -> Option<CapabilityStatus> {
        self.capabilities
            .get(capability_id)
            .map(Self::effective_status)
    }

    pub fn policy_enabled(&self, capability_id: &str) -> Option<bool> {
        self.capabilities
            .get(capability_id)
            .map(|record| record.policy_enabled)
    }

    pub fn set_policy_status(
        &mut self,
        capability_id: &str,
        enabled: bool,
    ) -> Result<CapabilityStatus, CapabilityError> {
        let record = self
            .capabilities
            .get_mut(capability_id)
            .ok_or(CapabilityError::NotFound)?;
        record.policy_enabled = enabled && record.status != CapabilityStatus::DisabledByPolicy;
        Ok(Self::effective_status(record))
    }

    pub fn supports(&self, capability_id: &str) -> bool {
        let Some(record) = self.capabilities.get(capability_id) else {
            return false;
        };
        if !record.policy_enabled {
            return false;
        }
        matches!(
            (self.engine, Some(record.status)),
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
            .map(|(id, record)| (id.as_str(), Self::effective_status(record)))
    }

    pub fn records(&self) -> impl Iterator<Item = (&str, &CapabilityRecord)> {
        self.capabilities
            .iter()
            .map(|(id, record)| (id.as_str(), record))
    }

    pub fn version(&self) -> &str {
        &self.snapshot_version
    }

    pub fn response(&self) -> CapabilityResponse {
        CapabilityResponse {
            engine: self.engine,
            engine_build: self.engine_build.clone(),
            snapshot_version: self.snapshot_version.clone(),
            capabilities: self
                .capabilities
                .iter()
                .map(|(id, record)| CapabilityEntry {
                    id: id.clone(),
                    status: Self::effective_status(record),
                    policy_enabled: record.policy_enabled,
                })
                .collect(),
        }
    }

    fn effective_status(record: &CapabilityRecord) -> CapabilityStatus {
        if record.policy_enabled {
            record.status
        } else {
            CapabilityStatus::DisabledByPolicy
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityError {
    NotFound,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapabilityRegistry {
    snapshots: Vec<CapabilitySnapshot>,
}

impl CapabilityRegistry {
    pub fn upsert(&mut self, snapshot: CapabilitySnapshot) -> Option<CapabilitySnapshot> {
        if let Some(index) = self
            .snapshots
            .iter()
            .position(|existing| existing.engine == snapshot.engine)
        {
            return Some(std::mem::replace(&mut self.snapshots[index], snapshot));
        }
        self.snapshots.push(snapshot);
        None
    }

    pub fn snapshot(&self, engine: EngineKind) -> Option<&CapabilitySnapshot> {
        self.snapshots
            .iter()
            .find(|snapshot| snapshot.engine == engine)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CapabilitySnapshot> {
        self.snapshots.iter()
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    pub fn responses(&self) -> impl Iterator<Item = CapabilityResponse> + '_ {
        self.snapshots.iter().map(CapabilitySnapshot::response)
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityRegistry, CapabilitySnapshot, CapabilityStatus, EngineKind};

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
        assert_eq!(
            snapshot.status("dom.query.v1"),
            Some(CapabilityStatus::DisabledByPolicy)
        );
    }

    #[test]
    fn stores_snapshot_versions_and_policy_overrides() {
        let mut snapshot =
            CapabilitySnapshot::versioned(EngineKind::Chromium, "chromium-123", "capability.v0.1");
        snapshot.register_with_policy("dom.query.v1", CapabilityStatus::Chromium, false);
        assert_eq!(snapshot.version(), "capability.v0.1");
        assert_eq!(snapshot.policy_enabled("dom.query.v1"), Some(false));
        assert_eq!(
            snapshot.set_policy_status("dom.query.v1", true),
            Ok(CapabilityStatus::Chromium)
        );
        assert!(snapshot.supports("dom.query.v1"));
        assert_eq!(
            snapshot.response().capabilities[0].status,
            CapabilityStatus::Chromium
        );
    }

    #[test]
    fn registry_replaces_stale_engine_snapshot() {
        let mut registry = CapabilityRegistry::default();
        registry.upsert(CapabilitySnapshot::new(EngineKind::Native, "native-1"));
        let previous = registry.upsert(CapabilitySnapshot::new(EngineKind::Native, "native-2"));
        assert_eq!(
            previous.map(|snapshot| snapshot.engine_build),
            Some("native-1".to_owned())
        );
        assert_eq!(registry.len(), 1);
    }
}
