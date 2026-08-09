//! Tenant-scoped classified artifacts and redacted reproduction bundles.
//!
//! The in-memory store models the durable contract: callers provide ciphertext
//! and a key reference, access is scope checked, integrity is hash checked, and
//! signed download grants are short lived. It never logs or decrypts payloads.

use std::collections::{BTreeMap, BTreeSet};

use hmac::{Hmac, Mac};
use machina_auth::ProjectScope;
use machina_command_model::DataClassification;
use machina_telemetry::{TraceEvent, ValidationError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub const REPRODUCTION_BUNDLE_VERSION: &str = "reproduction-bundle.v0";
pub const MAX_SIGNED_URL_TTL_SECONDS: u64 = 15 * 60;
pub const MAX_BUNDLE_JSON_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_BUNDLE_METADATA_ENTRIES: usize = 128;
pub const MAX_BUNDLE_TRACE_EVENTS: usize = 2_048;
pub const MAX_BUNDLE_ARTIFACTS: usize = 256;
const REDACTION_MARKER: &str = "[REDACTED]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactStoreLimits {
    pub max_artifact_bytes: usize,
    pub max_bundle_json_bytes: usize,
    pub max_bundle_metadata_entries: usize,
    pub max_bundle_trace_events: usize,
    pub max_bundle_artifacts: usize,
}

impl Default for ArtifactStoreLimits {
    fn default() -> Self {
        Self {
            max_artifact_bytes: 16 * 1024 * 1024,
            max_bundle_json_bytes: MAX_BUNDLE_JSON_BYTES,
            max_bundle_metadata_entries: MAX_BUNDLE_METADATA_ENTRIES,
            max_bundle_trace_events: MAX_BUNDLE_TRACE_EVENTS,
            max_bundle_artifacts: MAX_BUNDLE_ARTIFACTS,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactMetadata {
    pub artifact_id: String,
    pub organization_id: String,
    pub project_id: String,
    pub purpose: String,
    pub classification: DataClassification,
    pub storage_key: String,
    pub encryption_key_ref: String,
    pub sha256: String,
    pub byte_length: u64,
    pub created_at: u64,
    pub expires_at: u64,
}

impl ArtifactMetadata {
    pub fn scope(&self) -> Result<ProjectScope, ArtifactError> {
        ProjectScope::new(&self.organization_id, &self.project_id)
            .map_err(|_| ArtifactError::InvalidMetadata("artifact scope"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedArtifactUrl {
    pub artifact_id: String,
    pub organization_id: String,
    pub project_id: String,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactInput {
    pub artifact_id: String,
    pub purpose: String,
    pub classification: DataClassification,
    pub ciphertext: Vec<u8>,
    pub encryption_key_ref: String,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BundleArtifact {
    pub artifact_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub classification: DataClassification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReproductionBundle {
    pub schema_version: String,
    pub bundle_id: String,
    pub organization_id: String,
    pub project_id: String,
    pub generated_at: u64,
    pub metadata: BTreeMap<String, String>,
    pub trace: Vec<TraceEvent>,
    pub artifacts: Vec<BundleArtifact>,
    pub sha256: String,
}

impl ReproductionBundle {
    pub fn build(
        scope: &ProjectScope,
        bundle_id: impl Into<String>,
        generated_at: u64,
        metadata: BTreeMap<String, String>,
        trace: Vec<TraceEvent>,
        artifacts: Vec<BundleArtifact>,
    ) -> Result<Self, ArtifactError> {
        if metadata.len() > MAX_BUNDLE_METADATA_ENTRIES
            || trace.len() > MAX_BUNDLE_TRACE_EVENTS
            || artifacts.len() > MAX_BUNDLE_ARTIFACTS
        {
            return Err(ArtifactError::ResourceLimit("bundle size"));
        }
        let bundle_id = bundle_id.into();
        if bundle_id.trim().is_empty() {
            return Err(ArtifactError::InvalidMetadata("bundle_id"));
        }
        validate_redaction(&metadata, &trace)?;
        for event in &trace {
            event.validate().map_err(ArtifactError::InvalidTrace)?;
        }
        let mut bundle = Self {
            schema_version: REPRODUCTION_BUNDLE_VERSION.to_owned(),
            bundle_id,
            organization_id: scope.organization_id.clone(),
            project_id: scope.project_id.clone(),
            generated_at,
            metadata,
            trace,
            artifacts,
            sha256: String::new(),
        };
        bundle.sha256 = bundle.payload_hash()?;
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn validate(&self) -> Result<(), ArtifactError> {
        if self.schema_version != REPRODUCTION_BUNDLE_VERSION {
            return Err(ArtifactError::InvalidMetadata("bundle schema version"));
        }
        if self.bundle_id.trim().is_empty() {
            return Err(ArtifactError::InvalidMetadata("bundle_id"));
        }
        for value in [
            self.bundle_id.as_str(),
            self.organization_id.as_str(),
            self.project_id.as_str(),
        ] {
            if contains_canary(value) {
                return Err(ArtifactError::InvalidMetadata("canary secret"));
            }
        }
        ProjectScope::new(&self.organization_id, &self.project_id)
            .map_err(|_| ArtifactError::InvalidMetadata("bundle scope"))?;
        if self.metadata.len() > MAX_BUNDLE_METADATA_ENTRIES
            || self.trace.len() > MAX_BUNDLE_TRACE_EVENTS
            || self.artifacts.len() > MAX_BUNDLE_ARTIFACTS
        {
            return Err(ArtifactError::ResourceLimit("bundle size"));
        }
        validate_redaction(&self.metadata, &self.trace)?;
        for event in &self.trace {
            event.validate().map_err(ArtifactError::InvalidTrace)?;
        }
        for artifact in &self.artifacts {
            if artifact.artifact_id.trim().is_empty()
                || artifact.sha256.len() != 64
                || !artifact.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(ArtifactError::InvalidMetadata("bundle artifact"));
            }
            if contains_canary(&artifact.artifact_id) {
                return Err(ArtifactError::InvalidMetadata("canary secret"));
            }
        }
        if self.sha256 != self.payload_hash()? {
            return Err(ArtifactError::IntegrityFailure);
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<Vec<u8>, ArtifactError> {
        self.validate()?;
        let encoded = serde_json::to_vec(self).map_err(|_| ArtifactError::Serialization)?;
        if encoded.len() > MAX_BUNDLE_JSON_BYTES {
            return Err(ArtifactError::ResourceLimit("bundle JSON"));
        }
        Ok(encoded)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, ArtifactError> {
        if bytes.len() > MAX_BUNDLE_JSON_BYTES {
            return Err(ArtifactError::ResourceLimit("bundle JSON"));
        }
        let bundle: Self =
            serde_json::from_slice(bytes).map_err(|_| ArtifactError::Serialization)?;
        bundle.validate()?;
        Ok(bundle)
    }

    fn payload_hash(&self) -> Result<String, ArtifactError> {
        let mut payload = self.clone();
        payload.sha256.clear();
        let encoded = serde_json::to_vec(&payload).map_err(|_| ArtifactError::Serialization)?;
        Ok(hex_digest(&encoded))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactError {
    InvalidMetadata(&'static str),
    NotFound,
    AccessDenied,
    Expired,
    InvalidSignature,
    IntegrityFailure,
    InvalidTrace(ValidationError),
    ResourceLimit(&'static str),
    Serialization,
    DuplicateArtifact,
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetadata(field) => write!(formatter, "invalid artifact metadata: {field}"),
            Self::NotFound => formatter.write_str("artifact not found"),
            Self::AccessDenied => formatter.write_str("artifact access denied"),
            Self::Expired => formatter.write_str("artifact grant or object expired"),
            Self::InvalidSignature => formatter.write_str("artifact signature is invalid"),
            Self::IntegrityFailure => formatter.write_str("artifact integrity check failed"),
            Self::InvalidTrace(_) => formatter.write_str("trace event is invalid"),
            Self::ResourceLimit(field) => {
                write!(formatter, "artifact resource limit exceeded: {field}")
            }
            Self::Serialization => formatter.write_str("artifact serialization failed"),
            Self::DuplicateArtifact => formatter.write_str("artifact already exists"),
        }
    }
}

impl std::error::Error for ArtifactError {}

#[derive(Clone, Debug)]
struct StoredArtifact {
    metadata: ArtifactMetadata,
    ciphertext: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ArtifactStore {
    signing_key: Vec<u8>,
    limits: ArtifactStoreLimits,
    artifacts: BTreeMap<(String, String, String), StoredArtifact>,
}

impl ArtifactStore {
    pub fn new(signing_key: impl AsRef<[u8]>) -> Result<Self, ArtifactError> {
        Self::with_limits(signing_key, ArtifactStoreLimits::default())
    }

    pub fn with_limits(
        signing_key: impl AsRef<[u8]>,
        limits: ArtifactStoreLimits,
    ) -> Result<Self, ArtifactError> {
        let signing_key = signing_key.as_ref().to_vec();
        if signing_key.len() < 16 {
            return Err(ArtifactError::InvalidMetadata("signing key"));
        }
        Ok(Self {
            signing_key,
            limits,
            artifacts: BTreeMap::new(),
        })
    }

    pub fn put(
        &mut self,
        scope: &ProjectScope,
        input: ArtifactInput,
    ) -> Result<ArtifactMetadata, ArtifactError> {
        let ArtifactInput {
            artifact_id,
            purpose,
            classification,
            ciphertext,
            encryption_key_ref,
            created_at,
            expires_at,
        } = input;
        validate_scope_component(&scope.organization_id, "organization_id")?;
        validate_scope_component(&scope.project_id, "project_id")?;
        validate_scope_component(&artifact_id, "artifact_id")?;
        if contains_canary(&scope.organization_id)
            || contains_canary(&scope.project_id)
            || contains_canary(&artifact_id)
        {
            return Err(ArtifactError::InvalidMetadata("canary secret"));
        }
        if artifact_id.trim().is_empty()
            || purpose.trim().is_empty()
            || ciphertext.is_empty()
            || encryption_key_ref.trim().is_empty()
            || expires_at <= created_at
        {
            return Err(ArtifactError::InvalidMetadata("artifact fields"));
        }
        if ciphertext.len() > self.limits.max_artifact_bytes {
            return Err(ArtifactError::ResourceLimit("artifact bytes"));
        }
        let storage_identity = artifact_key(scope, &artifact_id);
        if self.artifacts.contains_key(&storage_identity) {
            return Err(ArtifactError::DuplicateArtifact);
        }
        let metadata = ArtifactMetadata {
            artifact_id: artifact_id.clone(),
            organization_id: scope.organization_id.clone(),
            project_id: scope.project_id.clone(),
            purpose,
            classification,
            storage_key: format!(
                "artifact/{}/{}/{}",
                scope.organization_id, scope.project_id, artifact_id
            ),
            encryption_key_ref,
            sha256: hex_digest(&ciphertext),
            byte_length: ciphertext.len() as u64,
            created_at,
            expires_at,
        };
        self.artifacts.insert(
            storage_identity,
            StoredArtifact {
                metadata: metadata.clone(),
                ciphertext,
            },
        );
        Ok(metadata)
    }

    pub fn metadata(
        &self,
        scope: &ProjectScope,
        artifact_id: &str,
    ) -> Result<ArtifactMetadata, ArtifactError> {
        Ok(self.record(scope, artifact_id)?.metadata.clone())
    }

    pub fn download(
        &self,
        scope: &ProjectScope,
        artifact_id: &str,
        now: u64,
    ) -> Result<Vec<u8>, ArtifactError> {
        let record = self.record(scope, artifact_id)?;
        if now >= record.metadata.expires_at {
            return Err(ArtifactError::Expired);
        }
        verify_integrity(record)?;
        Ok(record.ciphertext.clone())
    }

    pub fn issue_signed_url(
        &self,
        scope: &ProjectScope,
        artifact_id: &str,
        now: u64,
        ttl_seconds: u64,
    ) -> Result<SignedArtifactUrl, ArtifactError> {
        if ttl_seconds == 0 {
            return Err(ArtifactError::InvalidMetadata("signed URL TTL"));
        }
        let metadata = self.metadata(scope, artifact_id)?;
        if now >= metadata.expires_at {
            return Err(ArtifactError::Expired);
        }
        let expires_at = metadata
            .expires_at
            .min(now.saturating_add(ttl_seconds.min(MAX_SIGNED_URL_TTL_SECONDS)));
        let signature = self.sign(scope, artifact_id, expires_at)?;
        Ok(SignedArtifactUrl {
            artifact_id: artifact_id.to_owned(),
            organization_id: scope.organization_id.clone(),
            project_id: scope.project_id.clone(),
            expires_at,
            signature,
        })
    }

    pub fn resolve_signed_url(
        &self,
        scope: &ProjectScope,
        url: &SignedArtifactUrl,
        now: u64,
    ) -> Result<Vec<u8>, ArtifactError> {
        if now >= url.expires_at
            || url.organization_id != scope.organization_id
            || url.project_id != scope.project_id
        {
            return Err(if now >= url.expires_at {
                ArtifactError::Expired
            } else {
                ArtifactError::AccessDenied
            });
        }
        let expected = self.sign(scope, &url.artifact_id, url.expires_at)?;
        if !constant_time_equal(expected.as_bytes(), url.signature.as_bytes()) {
            return Err(ArtifactError::InvalidSignature);
        }
        self.download(scope, &url.artifact_id, now)
    }

    pub fn build_reproduction_bundle(
        &self,
        scope: &ProjectScope,
        bundle_id: impl Into<String>,
        generated_at: u64,
        metadata: BTreeMap<String, String>,
        trace: Vec<TraceEvent>,
        artifact_ids: &[String],
    ) -> Result<ReproductionBundle, ArtifactError> {
        if metadata.len() > self.limits.max_bundle_metadata_entries
            || trace.len() > self.limits.max_bundle_trace_events
            || artifact_ids.len() > self.limits.max_bundle_artifacts
        {
            return Err(ArtifactError::ResourceLimit("bundle size"));
        }
        let mut seen = BTreeSet::new();
        let mut artifacts = Vec::with_capacity(artifact_ids.len());
        for artifact_id in artifact_ids {
            if !seen.insert(artifact_id) {
                return Err(ArtifactError::InvalidMetadata("duplicate bundle artifact"));
            }
            let artifact = self.metadata(scope, artifact_id)?;
            artifacts.push(BundleArtifact {
                artifact_id: artifact.artifact_id,
                sha256: artifact.sha256,
                byte_length: artifact.byte_length,
                classification: artifact.classification,
            });
        }
        let bundle =
            ReproductionBundle::build(scope, bundle_id, generated_at, metadata, trace, artifacts)?;
        let encoded = serde_json::to_vec(&bundle).map_err(|_| ArtifactError::Serialization)?;
        if encoded.len() > self.limits.max_bundle_json_bytes {
            return Err(ArtifactError::ResourceLimit("bundle JSON"));
        }
        Ok(bundle)
    }

    fn record(
        &self,
        scope: &ProjectScope,
        artifact_id: &str,
    ) -> Result<&StoredArtifact, ArtifactError> {
        self.artifacts
            .get(&artifact_key(scope, artifact_id))
            .ok_or_else(|| {
                if self
                    .artifacts
                    .keys()
                    .any(|(_, _, existing_id)| existing_id == artifact_id)
                {
                    ArtifactError::AccessDenied
                } else {
                    ArtifactError::NotFound
                }
            })
    }

    fn sign(
        &self,
        scope: &ProjectScope,
        artifact_id: &str,
        expires_at: u64,
    ) -> Result<String, ArtifactError> {
        validate_scope_component(&scope.organization_id, "organization_id")?;
        validate_scope_component(&scope.project_id, "project_id")?;
        validate_scope_component(artifact_id, "artifact_id")?;
        let mut mac = HmacSha256::new_from_slice(&self.signing_key)
            .map_err(|_| ArtifactError::InvalidMetadata("signing key"))?;
        update_signed_component(&mut mac, &scope.organization_id);
        update_signed_component(&mut mac, &scope.project_id);
        update_signed_component(&mut mac, artifact_id);
        update_signed_component(&mut mac, &expires_at.to_string());
        Ok(hex_digest(mac.finalize().into_bytes()))
    }
}

fn verify_integrity(record: &StoredArtifact) -> Result<(), ArtifactError> {
    if record.metadata.byte_length != record.ciphertext.len() as u64
        || record.metadata.sha256 != hex_digest(&record.ciphertext)
    {
        return Err(ArtifactError::IntegrityFailure);
    }
    Ok(())
}

fn validate_redaction(
    metadata: &BTreeMap<String, String>,
    trace: &[TraceEvent],
) -> Result<(), ArtifactError> {
    for (key, value) in metadata {
        if contains_canary(key) {
            return Err(ArtifactError::InvalidMetadata("canary secret"));
        }
        let sensitive_key = key.to_ascii_lowercase().contains("secret")
            || key.to_ascii_lowercase().contains("token")
            || key.to_ascii_lowercase().contains("password")
            || key.to_ascii_lowercase().contains("cookie")
            || key.to_ascii_lowercase().contains("authorization");
        if sensitive_key && value != REDACTION_MARKER {
            return Err(ArtifactError::InvalidMetadata("unredacted metadata"));
        }
        if contains_canary(value) {
            return Err(ArtifactError::InvalidMetadata("canary secret"));
        }
    }
    if trace.iter().any(|event| {
        contains_canary(&event.event_id)
            || contains_canary(&event.event_type)
            || contains_canary(&event.timestamp)
            || contains_canary(&event.redacted_message)
            || contains_canary(&event.context.correlation_id)
            || event
                .context
                .causation_id
                .as_deref()
                .is_some_and(contains_canary)
            || event
                .context
                .task_id
                .as_deref()
                .is_some_and(contains_canary)
            || event
                .context
                .command_id
                .as_deref()
                .is_some_and(contains_canary)
            || event
                .context
                .session_id
                .as_deref()
                .is_some_and(contains_canary)
    }) {
        return Err(ArtifactError::InvalidMetadata("canary secret"));
    }
    Ok(())
}

fn contains_canary(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("canary")
        || lower.contains("machina-secret")
        || lower.contains("ghp_")
        || lower.contains("bearer ")
}

fn validate_scope_component(value: &str, field: &'static str) -> Result<(), ArtifactError> {
    if value
        .bytes()
        .any(|byte| byte.is_ascii_control() || matches!(byte, b'/' | b'\\' | b'?' | b'#'))
    {
        return Err(ArtifactError::InvalidMetadata(field));
    }
    Ok(())
}

fn artifact_key(scope: &ProjectScope, artifact_id: &str) -> (String, String, String) {
    (
        scope.organization_id.clone(),
        scope.project_id.clone(),
        artifact_id.to_owned(),
    )
}

fn update_signed_component(mac: &mut HmacSha256, value: &str) {
    mac.update(&(value.len() as u64).to_be_bytes());
    mac.update(value.as_bytes());
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ArtifactError, ArtifactStore, ReproductionBundle, REPRODUCTION_BUNDLE_VERSION};
    use machina_auth::ProjectScope;
    use machina_command_model::DataClassification;
    use machina_telemetry::{
        CorrelationContext, DataClassification as TraceClassification, SessionTrace,
    };
    use std::collections::BTreeMap;

    fn scope(organization: &str, project: &str) -> ProjectScope {
        ProjectScope::new(organization, project).expect("scope")
    }

    fn trace() -> Vec<machina_telemetry::TraceEvent> {
        let context = CorrelationContext {
            correlation_id: "corr-1".to_owned(),
            causation_id: None,
            task_id: None,
            command_id: Some("command-1".to_owned()),
            session_id: Some("session-1".to_owned()),
        };
        let mut trace = SessionTrace::new(context, 8).expect("trace");
        trace
            .append(
                "event-1",
                "api.request.v1",
                "2026-08-09T00:00:00Z",
                TraceClassification::Tenant,
                "request",
            )
            .expect("event");
        trace
            .append(
                "event-2",
                "worker.outcome.v1",
                "2026-08-09T00:00:01Z",
                TraceClassification::Tenant,
                "verified",
            )
            .expect("event");
        trace.events().cloned().collect()
    }

    #[test]
    fn enforces_scope_integrity_and_expiration() {
        let owner = scope("org-1", "project-1");
        let other = scope("org-2", "project-2");
        let mut store = ArtifactStore::new("signing-key-that-is-long-enough").expect("store");
        store
            .put(
                &owner,
                super::ArtifactInput {
                    artifact_id: "artifact-1".to_owned(),
                    purpose: "failure-bundle".to_owned(),
                    classification: DataClassification::Restricted,
                    ciphertext: b"ciphertext".to_vec(),
                    encryption_key_ref: "kms/key-1".to_owned(),
                    created_at: 100,
                    expires_at: 200,
                },
            )
            .expect("put");
        assert_eq!(
            store.download(&other, "artifact-1", 110),
            Err(ArtifactError::AccessDenied)
        );
        assert_eq!(
            store.download(&owner, "artifact-1", 200),
            Err(ArtifactError::Expired)
        );
    }

    #[test]
    fn signed_url_is_scoped_short_lived_and_tamper_evident() {
        let owner = scope("org-1", "project-1");
        let other = scope("org-2", "project-2");
        let mut store = ArtifactStore::new("signing-key-that-is-long-enough").expect("store");
        store
            .put(
                &owner,
                super::ArtifactInput {
                    artifact_id: "artifact-1".to_owned(),
                    purpose: "trace".to_owned(),
                    classification: DataClassification::Tenant,
                    ciphertext: b"ciphertext".to_vec(),
                    encryption_key_ref: "kms/key-1".to_owned(),
                    created_at: 100,
                    expires_at: 1000,
                },
            )
            .expect("put");
        let url = store
            .issue_signed_url(&owner, "artifact-1", 110, 30)
            .expect("signed URL");
        assert_eq!(url.expires_at, 140);
        assert_eq!(
            store.resolve_signed_url(&other, &url, 120),
            Err(ArtifactError::AccessDenied)
        );
        assert_eq!(
            store.resolve_signed_url(&owner, &url, 140),
            Err(ArtifactError::Expired)
        );
        let mut tampered = url.clone();
        tampered.signature.replace_range(0..2, "ff");
        assert_eq!(
            store.resolve_signed_url(&owner, &tampered, 120),
            Err(ArtifactError::InvalidSignature)
        );
        let control_scope = scope("org\0one", "project-1");
        assert_eq!(
            store.put(
                &control_scope,
                super::ArtifactInput {
                    artifact_id: "artifact-2".to_owned(),
                    purpose: "trace".to_owned(),
                    classification: DataClassification::Tenant,
                    ciphertext: b"ciphertext".to_vec(),
                    encryption_key_ref: "kms/key-1".to_owned(),
                    created_at: 100,
                    expires_at: 1000,
                }
            ),
            Err(ArtifactError::InvalidMetadata("organization_id"))
        );
    }

    #[test]
    fn reproduction_bundle_is_redacted_hashed_and_round_trippable() {
        let owner = scope("org-1", "project-1");
        let mut store = ArtifactStore::new("signing-key-that-is-long-enough").expect("store");
        let artifact = store
            .put(
                &owner,
                super::ArtifactInput {
                    artifact_id: "artifact-1".to_owned(),
                    purpose: "failure-bundle".to_owned(),
                    classification: DataClassification::Restricted,
                    ciphertext: b"ciphertext".to_vec(),
                    encryption_key_ref: "kms/key-1".to_owned(),
                    created_at: 100,
                    expires_at: 1000,
                },
            )
            .expect("put");
        let mut metadata = BTreeMap::new();
        metadata.insert("engine".to_owned(), "chromium".to_owned());
        metadata.insert("authorization".to_owned(), "[REDACTED]".to_owned());
        let bundle = store
            .build_reproduction_bundle(
                &owner,
                "bundle-1",
                120,
                metadata,
                trace(),
                &[artifact.artifact_id],
            )
            .expect("bundle");
        assert_eq!(bundle.schema_version, REPRODUCTION_BUNDLE_VERSION);
        let encoded = bundle.to_json().expect("json");
        let decoded = ReproductionBundle::from_json(&encoded).expect("round trip");
        assert_eq!(decoded, bundle);
        assert!(!String::from_utf8_lossy(&encoded).contains("ciphertext"));
        let mut secret_metadata = BTreeMap::new();
        secret_metadata.insert("token".to_owned(), "canary-secret".to_owned());
        assert_eq!(
            ReproductionBundle::build(
                &owner,
                "bundle-2",
                120,
                secret_metadata,
                Vec::new(),
                Vec::new()
            ),
            Err(ArtifactError::InvalidMetadata("unredacted metadata"))
        );
        let mut canary_trace = trace();
        canary_trace[0].context.correlation_id = "canary-secret".to_owned();
        assert_eq!(
            ReproductionBundle::build(
                &owner,
                "bundle-3",
                120,
                BTreeMap::new(),
                canary_trace,
                Vec::new()
            ),
            Err(ArtifactError::InvalidMetadata("canary secret"))
        );
        let mut unknown = serde_json::from_slice::<serde_json::Value>(&encoded).expect("value");
        unknown
            .as_object_mut()
            .expect("object")
            .insert("unexpected".to_owned(), serde_json::json!("canary-secret"));
        assert_eq!(
            ReproductionBundle::from_json(
                &serde_json::to_vec(&unknown).expect("unknown field JSON")
            ),
            Err(ArtifactError::Serialization)
        );
    }

    #[test]
    fn rejects_oversized_objects_before_storage() {
        let owner = scope("org-1", "project-1");
        let mut store = ArtifactStore::with_limits(
            "signing-key-that-is-long-enough",
            super::ArtifactStoreLimits {
                max_artifact_bytes: 4,
                ..super::ArtifactStoreLimits::default()
            },
        )
        .expect("store");
        assert_eq!(
            store.put(
                &owner,
                super::ArtifactInput {
                    artifact_id: "artifact-1".to_owned(),
                    purpose: "trace".to_owned(),
                    classification: DataClassification::Tenant,
                    ciphertext: b"12345".to_vec(),
                    encryption_key_ref: "kms/key-1".to_owned(),
                    created_at: 100,
                    expires_at: 1000,
                }
            ),
            Err(ArtifactError::ResourceLimit("artifact bytes"))
        );
    }
}
