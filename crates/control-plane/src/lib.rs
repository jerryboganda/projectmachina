//! Tenant-scoped control-plane primitives.
//!
//! The production store will use PostgreSQL migrations under `migrations/`.
//! This dependency-free implementation provides the same repository contract
//! and transaction/idempotency behavior for fast unit and contract tests.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TenantScope {
    pub organization_id: String,
    pub project_id: String,
}

impl TenantScope {
    pub fn new(
        organization_id: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Result<Self, StoreError> {
        let organization_id = organization_id.into();
        let project_id = project_id.into();
        if organization_id.trim().is_empty() || project_id.trim().is_empty() {
            return Err(StoreError::InvalidTenantScope);
        }
        Ok(Self {
            organization_id,
            project_id,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationContext {
    pub actor_id: String,
    pub scope: TenantScope,
    pub policy_hash: String,
}

impl AuthorizationContext {
    pub fn new(
        actor_id: impl Into<String>,
        scope: TenantScope,
        policy_hash: impl Into<String>,
    ) -> Result<Self, StoreError> {
        let actor_id = actor_id.into();
        let policy_hash = policy_hash.into();
        if actor_id.trim().is_empty() || policy_hash.trim().is_empty() {
            return Err(StoreError::InvalidArgument("authorization context"));
        }
        Ok(Self {
            actor_id,
            scope,
            policy_hash,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState {
    Requested,
    Queued,
    Starting,
    Ready,
    Closing,
    Closed,
    Failed,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub id: String,
    pub scope: TenantScope,
    pub policy_version: String,
    pub state: SessionState,
    pub version: u64,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxEvent {
    pub event_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub aggregate_version: u64,
    pub scope: TenantScope,
    pub event_type: String,
    pub payload_json: String,
    pub classification: String,
    pub created_at: String,
    pub published_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    InvalidTenantScope,
    InvalidArgument(&'static str),
    DuplicateIdempotencyKey,
    SessionNotFound,
    TenantAccessDenied,
    VersionConflict {
        expected: u64,
        actual: u64,
    },
    InvalidTransition {
        from: SessionState,
        to: SessionState,
    },
}

impl Display for StoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTenantScope => {
                formatter.write_str("organization and project are required")
            }
            Self::InvalidArgument(field) => write!(formatter, "invalid argument: {field}"),
            Self::DuplicateIdempotencyKey => formatter.write_str("idempotency key is already used"),
            Self::SessionNotFound => formatter.write_str("session not found"),
            Self::TenantAccessDenied => formatter.write_str("tenant access denied"),
            Self::VersionConflict { expected, actual } => {
                write!(
                    formatter,
                    "version conflict: expected {expected}, actual {actual}"
                )
            }
            Self::InvalidTransition { from, to } => {
                write!(formatter, "invalid session transition: {from:?} -> {to:?}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

pub trait ControlPlaneRepository {
    fn create_session(
        &mut self,
        authorization: AuthorizationContext,
        session_id: String,
        policy_version: String,
        idempotency_key: String,
        created_at: String,
    ) -> Result<SessionRecord, StoreError>;

    fn get_session(
        &self,
        authorization: &AuthorizationContext,
        session_id: &str,
    ) -> Result<SessionRecord, StoreError>;

    fn transition_session(
        &mut self,
        authorization: &AuthorizationContext,
        session_id: &str,
        expected_version: u64,
        next_state: SessionState,
        created_at: String,
    ) -> Result<SessionRecord, StoreError>;

    fn outbox_for(
        &self,
        authorization: &AuthorizationContext,
    ) -> Result<Vec<OutboxEvent>, StoreError>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryControlPlane {
    sessions: BTreeMap<String, SessionRecord>,
    idempotency: BTreeMap<(String, String, String), String>,
    outbox: Vec<OutboxEvent>,
    next_event_id: u64,
}

impl InMemoryControlPlane {
    fn append_event(
        &mut self,
        session: &SessionRecord,
        event_type: &str,
        payload_json: String,
        created_at: String,
    ) {
        self.next_event_id += 1;
        self.outbox.push(OutboxEvent {
            event_id: format!("event-{}", self.next_event_id),
            aggregate_type: "session".to_owned(),
            aggregate_id: session.id.clone(),
            aggregate_version: session.version,
            scope: session.scope.clone(),
            event_type: event_type.to_owned(),
            payload_json,
            classification: "tenant".to_owned(),
            created_at,
            published_at: None,
        });
    }

    fn session_for_scope(
        &self,
        scope: &TenantScope,
        session_id: &str,
    ) -> Result<&SessionRecord, StoreError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or(StoreError::SessionNotFound)?;
        if &session.scope != scope {
            return Err(StoreError::TenantAccessDenied);
        }
        Ok(session)
    }
}

impl ControlPlaneRepository for InMemoryControlPlane {
    fn create_session(
        &mut self,
        authorization: AuthorizationContext,
        session_id: String,
        policy_version: String,
        idempotency_key: String,
        created_at: String,
    ) -> Result<SessionRecord, StoreError> {
        if session_id.trim().is_empty() {
            return Err(StoreError::InvalidArgument("session_id"));
        }
        if policy_version.trim().is_empty() {
            return Err(StoreError::InvalidArgument("policy_version"));
        }
        if idempotency_key.trim().is_empty() {
            return Err(StoreError::InvalidArgument("idempotency_key"));
        }

        let scope = authorization.scope;
        let mut candidate = self.clone();
        if let Some(existing_id) = candidate.idempotency.get(&(
            scope.organization_id.clone(),
            scope.project_id.clone(),
            idempotency_key.clone(),
        )) {
            if existing_id == &session_id {
                return candidate
                    .sessions
                    .get(existing_id)
                    .cloned()
                    .ok_or(StoreError::SessionNotFound);
            }
            return Err(StoreError::DuplicateIdempotencyKey);
        }
        if candidate.sessions.contains_key(&session_id) {
            return Err(StoreError::DuplicateIdempotencyKey);
        }

        let record = SessionRecord {
            id: session_id.clone(),
            scope: scope.clone(),
            policy_version,
            state: SessionState::Requested,
            version: 1,
            idempotency_key: idempotency_key.clone(),
        };
        candidate.idempotency.insert(
            (scope.organization_id, scope.project_id, idempotency_key),
            session_id.clone(),
        );
        candidate.sessions.insert(session_id, record.clone());
        candidate.append_event(
            &record,
            "session.requested.v1",
            "{\"state\":\"requested\"}".to_owned(),
            created_at,
        );
        *self = candidate;
        Ok(record)
    }

    fn get_session(
        &self,
        authorization: &AuthorizationContext,
        session_id: &str,
    ) -> Result<SessionRecord, StoreError> {
        self.session_for_scope(&authorization.scope, session_id)
            .cloned()
    }

    fn transition_session(
        &mut self,
        authorization: &AuthorizationContext,
        session_id: &str,
        expected_version: u64,
        next_state: SessionState,
        created_at: String,
    ) -> Result<SessionRecord, StoreError> {
        let mut candidate = self.clone();
        let current = candidate
            .session_for_scope(&authorization.scope, session_id)?
            .clone();
        if current.version != expected_version {
            return Err(StoreError::VersionConflict {
                expected: expected_version,
                actual: current.version,
            });
        }
        let valid_transition = matches!(
            (current.state, next_state),
            (SessionState::Requested, SessionState::Queued)
                | (SessionState::Requested, SessionState::Failed)
                | (SessionState::Requested, SessionState::Closing)
                | (SessionState::Requested, SessionState::Ready)
                | (SessionState::Queued, SessionState::Starting)
                | (SessionState::Queued, SessionState::Failed)
                | (SessionState::Queued, SessionState::Closing)
                | (SessionState::Starting, SessionState::Ready)
                | (SessionState::Starting, SessionState::Failed)
                | (SessionState::Starting, SessionState::Closing)
                | (SessionState::Ready, SessionState::Closing)
                | (SessionState::Ready, SessionState::Failed)
                | (SessionState::Ready, SessionState::Expired)
                | (SessionState::Closing, SessionState::Closed)
        );
        if !valid_transition {
            return Err(StoreError::InvalidTransition {
                from: current.state,
                to: next_state,
            });
        }
        let mut next = current;
        next.state = next_state;
        next.version += 1;
        candidate
            .sessions
            .insert(session_id.to_owned(), next.clone());
        candidate.append_event(
            &next,
            "session.state_changed.v1",
            format!("{{\"state\":\"{next_state:?}\"}}"),
            created_at,
        );
        *self = candidate;
        Ok(next)
    }

    fn outbox_for(
        &self,
        authorization: &AuthorizationContext,
    ) -> Result<Vec<OutboxEvent>, StoreError> {
        Ok(self
            .outbox
            .iter()
            .filter(|event| event.scope == authorization.scope)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AuthorizationContext, ControlPlaneRepository, InMemoryControlPlane, SessionState,
        StoreError, TenantScope,
    };

    fn scope(project: &str) -> TenantScope {
        TenantScope::new("org-1", project).expect("valid scope")
    }

    fn authorization(project: &str) -> AuthorizationContext {
        AuthorizationContext::new("actor-1", scope(project), "policy-hash-1")
            .expect("valid authorization context")
    }

    #[test]
    fn aggregate_and_outbox_commit_atomically() {
        let mut store = InMemoryControlPlane::default();
        let authorization = authorization("project-1");
        let session = store
            .create_session(
                authorization.clone(),
                "session-1".to_owned(),
                "policy-v1".to_owned(),
                "idem-1".to_owned(),
                "2026-08-09T00:00:00Z".to_owned(),
            )
            .expect("create");
        assert_eq!(session.version, 1);
        assert_eq!(store.outbox_for(&authorization).expect("outbox").len(), 1);
        let ready = store
            .transition_session(
                &authorization,
                "session-1",
                1,
                SessionState::Ready,
                "2026-08-09T00:00:01Z".to_owned(),
            )
            .expect("transition");
        assert_eq!(ready.version, 2);
        assert_eq!(store.outbox_for(&authorization).expect("outbox").len(), 2);
    }

    #[test]
    fn rejects_cross_tenant_reads_and_stale_versions_without_mutation() {
        let mut store = InMemoryControlPlane::default();
        let owner = authorization("project-1");
        store
            .create_session(
                owner.clone(),
                "session-1".to_owned(),
                "policy-v1".to_owned(),
                "idem-1".to_owned(),
                "now".to_owned(),
            )
            .expect("create");
        assert_eq!(
            store.get_session(&authorization("project-2"), "session-1"),
            Err(StoreError::TenantAccessDenied)
        );
        assert!(matches!(
            store.transition_session(
                &owner,
                "session-1",
                99,
                SessionState::Ready,
                "now".to_owned()
            ),
            Err(StoreError::VersionConflict { .. })
        ));
        assert!(matches!(
            store.transition_session(
                &owner,
                "session-1",
                1,
                SessionState::Closed,
                "now".to_owned()
            ),
            Err(StoreError::InvalidTransition { .. })
        ));
        assert!(store
            .outbox_for(&authorization("project-2"))
            .expect("scoped outbox")
            .is_empty());
        assert_eq!(store.outbox_for(&owner).expect("outbox").len(), 1);
    }

    #[test]
    fn idempotent_create_replays_same_outcome_and_rejects_key_reuse() {
        let mut store = InMemoryControlPlane::default();
        let owner = authorization("project-1");
        let first = store
            .create_session(
                owner.clone(),
                "session-1".to_owned(),
                "policy-v1".to_owned(),
                "idem-1".to_owned(),
                "now".to_owned(),
            )
            .expect("create");
        let replay = store
            .create_session(
                owner.clone(),
                "session-1".to_owned(),
                "policy-v1".to_owned(),
                "idem-1".to_owned(),
                "now".to_owned(),
            )
            .expect("idempotent replay");
        assert_eq!(first, replay);
        assert!(store
            .create_session(
                authorization("project-2"),
                "session-2".to_owned(),
                "policy-v1".to_owned(),
                "idem-1".to_owned(),
                "now".to_owned()
            )
            .is_ok());
        assert_eq!(
            store
                .outbox_for(&authorization("project-1"))
                .expect("outbox")
                .len(),
            1
        );
    }
}
