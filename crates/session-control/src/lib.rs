//! Idempotent session lifecycle service.
//!
//! Transport adapters call this service; workers never write the control-plane
//! database directly. State transitions require the expected aggregate version.

use machina_control_plane::{
    AuthorizationContext, ControlPlaneRepository, SessionRecord, SessionState, StoreError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateSessionRequest {
    pub session_id: String,
    pub policy_version: String,
    pub idempotency_key: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct SessionService<R> {
    repository: R,
}

impl<R> SessionService<R>
where
    R: ControlPlaneRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn repository_mut(&mut self) -> &mut R {
        &mut self.repository
    }

    pub fn create(
        &mut self,
        authorization: AuthorizationContext,
        request: CreateSessionRequest,
    ) -> Result<SessionRecord, StoreError> {
        self.repository.create_session(
            authorization,
            request.session_id,
            request.policy_version,
            request.idempotency_key,
            request.created_at,
        )
    }

    pub fn get(
        &self,
        authorization: &AuthorizationContext,
        session_id: &str,
    ) -> Result<SessionRecord, StoreError> {
        self.repository.get_session(authorization, session_id)
    }

    pub fn cancel(
        &mut self,
        authorization: &AuthorizationContext,
        session_id: &str,
        expected_version: u64,
        now: String,
    ) -> Result<SessionRecord, StoreError> {
        let current = self.get(authorization, session_id)?;
        if matches!(current.state, SessionState::Failed | SessionState::Closed) {
            return Ok(current);
        }
        self.repository.transition_session(
            authorization,
            session_id,
            expected_version,
            SessionState::Failed,
            now,
        )
    }

    pub fn close(
        &mut self,
        authorization: &AuthorizationContext,
        session_id: &str,
        expected_version: u64,
        now: String,
    ) -> Result<SessionRecord, StoreError> {
        let current = self.get(authorization, session_id)?;
        if current.state == SessionState::Closed {
            return Ok(current);
        }
        let closing = self.repository.transition_session(
            authorization,
            session_id,
            expected_version,
            SessionState::Closing,
            now.clone(),
        )?;
        self.repository.transition_session(
            authorization,
            session_id,
            closing.version,
            SessionState::Closed,
            now,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateSessionRequest, SessionService};
    use machina_control_plane::{
        AuthorizationContext, InMemoryControlPlane, SessionState, StoreError, TenantScope,
    };

    fn auth() -> AuthorizationContext {
        AuthorizationContext::new(
            "actor-1",
            TenantScope::new("org-1", "project-1").expect("scope"),
            "policy-hash",
        )
        .expect("authorization")
    }

    fn request() -> CreateSessionRequest {
        CreateSessionRequest {
            session_id: "session-1".to_owned(),
            policy_version: "policy-v1".to_owned(),
            idempotency_key: "idem-1".to_owned(),
            created_at: "2026-08-09T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn create_is_idempotent_and_close_is_versioned() {
        let mut service = SessionService::new(InMemoryControlPlane::default());
        let first = service.create(auth(), request()).expect("create");
        let replay = service.create(auth(), request()).expect("replay");
        assert_eq!(first, replay);
        let closed = service
            .close(&auth(), "session-1", first.version, "now".to_owned())
            .expect("close");
        assert_eq!(closed.state, SessionState::Closed);
        assert_eq!(closed.version, 3);
    }

    #[test]
    fn cancel_and_cross_tenant_read_fail_safely() {
        let mut service = SessionService::new(InMemoryControlPlane::default());
        let created = service.create(auth(), request()).expect("create");
        let cancelled = service
            .cancel(&auth(), "session-1", created.version, "now".to_owned())
            .expect("cancel");
        assert_eq!(cancelled.state, SessionState::Failed);
        let other = AuthorizationContext::new(
            "actor-2",
            TenantScope::new("org-2", "project-2").expect("scope"),
            "policy-hash",
        )
        .expect("authorization");
        assert_eq!(
            service.get(&other, "session-1"),
            Err(StoreError::TenantAccessDenied)
        );
    }
}
