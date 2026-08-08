//! Lifecycle and bounded-resource primitives for browser sessions.
//!
//! Session state is independent from transport and engine implementation.
//! Limits fail explicitly before an operation can exceed its budget.

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use machina_command_bus::CancellationToken;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, SessionError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SessionError::InvalidIdentity(
                "session id must not be empty",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PageId(String);

impl PageId {
    pub fn new(value: impl Into<String>) -> Result<Self, SessionError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SessionError::InvalidIdentity("page id must not be empty"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionState {
    Creating,
    Ready,
    Closing,
    Closed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceBudget {
    pub max_pages: u32,
    pub max_requests: u64,
    pub max_bytes: u64,
    pub max_artifacts: u32,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_pages: 8,
            max_requests: 1_000,
            max_bytes: 64 * 1024 * 1024,
            max_artifacts: 100,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceUsage {
    pub pages: u32,
    pub requests: u64,
    pub bytes: u64,
    pub artifacts: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    Pages,
    Requests,
    Bytes,
    Artifacts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    InvalidIdentity(&'static str),
    InvalidTransition {
        from: SessionState,
        to: SessionState,
    },
    SessionNotReady,
    SessionClosed,
    PageAlreadyExists,
    PageNotFound,
    ResourceLimitExceeded {
        kind: ResourceKind,
        limit: u64,
    },
}

impl Display for SessionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity(message) => formatter.write_str(message),
            Self::InvalidTransition { from, to } => {
                write!(formatter, "invalid session transition: {from:?} -> {to:?}")
            }
            Self::SessionNotReady => formatter.write_str("session is not ready"),
            Self::SessionClosed => formatter.write_str("session is closed"),
            Self::PageAlreadyExists => formatter.write_str("page already exists"),
            Self::PageNotFound => formatter.write_str("page not found"),
            Self::ResourceLimitExceeded { kind, limit } => {
                write!(formatter, "resource limit exceeded for {kind:?}: {limit}")
            }
        }
    }
}

impl std::error::Error for SessionError {}

pub struct Session {
    id: SessionId,
    state: SessionState,
    budget: ResourceBudget,
    usage: ResourceUsage,
    pages: BTreeSet<PageId>,
    cancellation: CancellationToken,
}

impl Session {
    pub fn new(id: SessionId, budget: ResourceBudget) -> Self {
        Self {
            id,
            state: SessionState::Creating,
            budget,
            usage: ResourceUsage::default(),
            pages: BTreeSet::new(),
            cancellation: CancellationToken::new(),
        }
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn budget(&self) -> ResourceBudget {
        self.budget
    }

    pub fn usage(&self) -> ResourceUsage {
        self.usage
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn transition(&mut self, next: SessionState) -> Result<(), SessionError> {
        let valid = matches!(
            (self.state, next),
            (SessionState::Creating, SessionState::Ready)
                | (SessionState::Creating, SessionState::Failed)
                | (SessionState::Ready, SessionState::Closing)
                | (SessionState::Ready, SessionState::Failed)
                | (SessionState::Closing, SessionState::Closed)
                | (SessionState::Failed, SessionState::Closed)
        );
        if !valid {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        if matches!(
            next,
            SessionState::Closing | SessionState::Closed | SessionState::Failed
        ) {
            self.cancellation.cancel();
        }
        Ok(())
    }

    pub fn open_page(&mut self, page_id: PageId) -> Result<(), SessionError> {
        if self.state != SessionState::Ready {
            return Err(SessionError::SessionNotReady);
        }
        if self.pages.contains(&page_id) {
            return Err(SessionError::PageAlreadyExists);
        }
        if self.usage.pages >= self.budget.max_pages {
            return Err(SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Pages,
                limit: u64::from(self.budget.max_pages),
            });
        }
        self.pages.insert(page_id);
        self.usage.pages += 1;
        Ok(())
    }

    pub fn close_page(&mut self, page_id: &PageId) -> Result<(), SessionError> {
        if !self.pages.remove(page_id) {
            return Err(SessionError::PageNotFound);
        }
        self.usage.pages = self.usage.pages.saturating_sub(1);
        Ok(())
    }

    pub fn account_request(&mut self, bytes: u64) -> Result<(), SessionError> {
        if self.state != SessionState::Ready {
            return Err(SessionError::SessionNotReady);
        }
        let requests =
            self.usage
                .requests
                .checked_add(1)
                .ok_or(SessionError::ResourceLimitExceeded {
                    kind: ResourceKind::Requests,
                    limit: self.budget.max_requests,
                })?;
        if requests > self.budget.max_requests {
            return Err(SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Requests,
                limit: self.budget.max_requests,
            });
        }
        let total_bytes =
            self.usage
                .bytes
                .checked_add(bytes)
                .ok_or(SessionError::ResourceLimitExceeded {
                    kind: ResourceKind::Bytes,
                    limit: self.budget.max_bytes,
                })?;
        if total_bytes > self.budget.max_bytes {
            return Err(SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Bytes,
                limit: self.budget.max_bytes,
            });
        }
        self.usage.requests = requests;
        self.usage.bytes = total_bytes;
        Ok(())
    }

    pub fn account_artifact(&mut self) -> Result<(), SessionError> {
        let artifacts =
            self.usage
                .artifacts
                .checked_add(1)
                .ok_or(SessionError::ResourceLimitExceeded {
                    kind: ResourceKind::Artifacts,
                    limit: u64::from(self.budget.max_artifacts),
                })?;
        if artifacts > self.budget.max_artifacts {
            return Err(SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Artifacts,
                limit: u64::from(self.budget.max_artifacts),
            });
        }
        self.usage.artifacts = artifacts;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PageId, ResourceBudget, ResourceKind, Session, SessionError, SessionId, SessionState,
    };

    fn session() -> Session {
        Session::new(
            SessionId::new("session-1").expect("valid test id"),
            ResourceBudget {
                max_pages: 1,
                max_requests: 1,
                max_bytes: 10,
                max_artifacts: 1,
            },
        )
    }

    #[test]
    fn enforces_lifecycle_and_resource_limits() {
        let mut session = session();
        assert_eq!(session.state(), SessionState::Creating);
        session
            .transition(SessionState::Ready)
            .expect("creating -> ready");
        session
            .open_page(PageId::new("page-1").expect("valid page id"))
            .expect("first page fits");
        let error = session
            .open_page(PageId::new("page-2").expect("valid page id"))
            .expect_err("second page exceeds budget");
        assert!(matches!(
            error,
            SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Pages,
                ..
            }
        ));
        session.account_request(10).expect("request fits");
        assert!(session.account_request(1).is_err());
    }

    #[test]
    fn closing_cancels_work_and_rejects_new_pages() {
        let mut session = session();
        session
            .transition(SessionState::Ready)
            .expect("creating -> ready");
        session
            .transition(SessionState::Closing)
            .expect("ready -> closing");
        assert!(session.cancellation().is_cancelled());
        assert_eq!(
            session
                .open_page(PageId::new("page-1").expect("valid page id"))
                .expect_err("closing session cannot open page"),
            SessionError::SessionNotReady
        );
        session
            .transition(SessionState::Closed)
            .expect("closing -> closed");
    }

    #[test]
    fn invalid_transition_is_reported() {
        let mut session = session();
        let error = session
            .transition(SessionState::Closed)
            .expect_err("creating -> closed is not direct");
        assert!(matches!(error, SessionError::InvalidTransition { .. }));
    }
}
