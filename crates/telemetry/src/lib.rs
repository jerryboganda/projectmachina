//! Redaction-aware telemetry and deterministic evidence primitives.
//!
//! This crate deliberately stores references and classifications rather than
//! page content or secret values. Producers must redact before constructing
//! restricted diagnostic text.

use std::collections::BTreeSet;
use std::path::Path;

pub use machina_command_model::DataClassification;

pub const EVIDENCE_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrelationContext {
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub task_id: Option<String>,
    pub command_id: Option<String>,
    pub session_id: Option<String>,
}

impl CorrelationContext {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.correlation_id.trim().is_empty() {
            return Err(ValidationError::MissingField("correlation_id"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub event_id: String,
    pub sequence: u64,
    pub event_type: String,
    pub timestamp: String,
    pub context: CorrelationContext,
    pub classification: DataClassification,
    pub redacted_message: String,
}

impl TraceEvent {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.event_id.trim().is_empty() {
            return Err(ValidationError::MissingField("event_id"));
        }
        if self.sequence == 0 {
            return Err(ValidationError::InvalidValue("sequence must be positive"));
        }
        if self.event_type.trim().is_empty() {
            return Err(ValidationError::MissingField("event_type"));
        }
        self.context.validate()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricSample {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: String,
    pub context: CorrelationContext,
}

impl MetricSample {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.trim().is_empty() {
            return Err(ValidationError::MissingField("name"));
        }
        if !self.value.is_finite() {
            return Err(ValidationError::InvalidValue("metric value must be finite"));
        }
        if self.unit.trim().is_empty() {
            return Err(ValidationError::MissingField("unit"));
        }
        self.context.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceArtifact {
    pub artifact_id: String,
    pub relative_path: String,
    pub sha256: String,
    pub byte_length: u64,
    pub classification: DataClassification,
}

impl EvidenceArtifact {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.artifact_id.trim().is_empty() {
            return Err(ValidationError::MissingField("artifact_id"));
        }
        let path = Path::new(&self.relative_path);
        let has_parent_segment = self
            .relative_path
            .split(['/', '\\'])
            .any(|segment| segment == "..");
        if self.relative_path.trim().is_empty()
            || path.is_absolute()
            || self.relative_path.contains(':')
            || has_parent_segment
        {
            return Err(ValidationError::InvalidValue(
                "artifact path must be non-empty and repository-relative",
            ));
        }
        if self.sha256.len() != 64 || !self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ValidationError::InvalidValue(
                "sha256 must be 64 hexadecimal characters",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceManifest {
    pub schema_version: String,
    pub task_id: String,
    pub source_commit: String,
    pub generated_at: String,
    pub artifacts: Vec<EvidenceArtifact>,
}

impl EvidenceManifest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != EVIDENCE_SCHEMA_VERSION {
            return Err(ValidationError::InvalidValue(
                "unsupported evidence schema version",
            ));
        }
        if self.task_id.trim().is_empty() {
            return Err(ValidationError::MissingField("task_id"));
        }
        if self.source_commit.trim().is_empty() {
            return Err(ValidationError::MissingField("source_commit"));
        }
        if self.generated_at.trim().is_empty() {
            return Err(ValidationError::MissingField("generated_at"));
        }

        let mut paths = BTreeSet::new();
        for artifact in &self.artifacts {
            artifact.validate()?;
            if !paths.insert(&artifact.relative_path) {
                return Err(ValidationError::InvalidValue(
                    "evidence artifact paths must be unique",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    MissingField(&'static str),
    InvalidValue(&'static str),
}

#[cfg(test)]
mod tests {
    use super::{
        CorrelationContext, DataClassification, EvidenceArtifact, EvidenceManifest,
        EVIDENCE_SCHEMA_VERSION,
    };

    fn context() -> CorrelationContext {
        CorrelationContext {
            correlation_id: "corr-1".to_owned(),
            causation_id: None,
            task_id: Some("M0-T09".to_owned()),
            command_id: None,
            session_id: None,
        }
    }

    #[test]
    fn accepts_redacted_trace_context() {
        let trace = super::TraceEvent {
            event_id: "event-1".to_owned(),
            sequence: 1,
            event_type: "task.started.v1".to_owned(),
            timestamp: "2026-08-09T00:00:00Z".to_owned(),
            context: context(),
            classification: DataClassification::Restricted,
            redacted_message: "[REDACTED]".to_owned(),
        };
        assert!(trace.validate().is_ok());
    }

    #[test]
    fn rejects_duplicate_or_invalid_evidence_paths() {
        let artifact = |path: &str| EvidenceArtifact {
            artifact_id: path.to_owned(),
            relative_path: path.to_owned(),
            sha256: "a".repeat(64),
            byte_length: 1,
            classification: DataClassification::Restricted,
        };
        let manifest = EvidenceManifest {
            schema_version: EVIDENCE_SCHEMA_VERSION.to_owned(),
            task_id: "M0-T09".to_owned(),
            source_commit: "local-uncommitted".to_owned(),
            generated_at: "2026-08-09T00:00:00Z".to_owned(),
            artifacts: vec![artifact("evidence.json"), artifact("evidence.json")],
        };
        assert!(manifest.validate().is_err());
        assert!(artifact("../secret").validate().is_err());
        assert!(artifact("C:\\secret").validate().is_err());
    }
}
