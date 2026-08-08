//! Engine adapter foundation shared by native and Chromium worker paths.
//!
//! Only session lifecycle is enabled in this slice. Every other command
//! returns a typed unsupported-capability error; it is never treated as a
//! successful no-op.

use std::collections::BTreeMap;
use std::sync::Mutex;

use machina_capability::CapabilitySnapshot;
use machina_command_bus::{CommandContext, DispatchError, EngineAdapter};
use machina_command_model::{
    CapabilityStatus, CommandEnvelope, CommandKind, CommandPayload, EngineKind,
};
use machina_session::{ResourceBudget, Session, SessionId, SessionState};

struct LifecycleEngine {
    kind: EngineKind,
    snapshot: CapabilitySnapshot,
    sessions: Mutex<BTreeMap<SessionId, Session>>,
}

impl LifecycleEngine {
    fn new(kind: EngineKind, build: impl Into<String>) -> Self {
        let mut snapshot = CapabilitySnapshot::new(kind, build);
        let status = match kind {
            EngineKind::Native => CapabilityStatus::Native,
            EngineKind::Chromium => CapabilityStatus::Chromium,
        };
        snapshot.register("session.create.v1", status);
        snapshot.register("session.close.v1", status);
        Self {
            kind,
            snapshot,
            sessions: Mutex::new(BTreeMap::new()),
        }
    }

    fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<String, DispatchError> {
        if let Some(code) = context.is_cancelled_or_expired() {
            return Err(DispatchError::failed(code, format!("{code:?}"), false));
        }

        match command.kind {
            CommandKind::SessionCreateV1 => {
                if !matches!(command.payload, CommandPayload::SessionCreate(_)) {
                    return Err(DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::InvalidArgument,
                        "session.create.v1 requires a session-create payload",
                        false,
                    ));
                }
                let id = SessionId::new(command.session_id.clone()).map_err(|error| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::InvalidArgument,
                        error.to_string(),
                        false,
                    )
                })?;
                let mut sessions = self.sessions.lock().map_err(|_| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::CapacityUnavailable,
                        "session registry lock is poisoned",
                        true,
                    )
                })?;
                if sessions.contains_key(&id) {
                    return Err(DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::InvalidArgument,
                        "session already exists",
                        false,
                    ));
                }
                let mut session = Session::new(id.clone(), ResourceBudget::default());
                session.transition(SessionState::Ready).map_err(|error| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::SessionNotReady,
                        error.to_string(),
                        false,
                    )
                })?;
                sessions.insert(id, session);
                Ok("session ready".to_owned())
            }
            CommandKind::SessionCloseV1 => {
                if !matches!(command.payload, CommandPayload::SessionClose(_)) {
                    return Err(DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::InvalidArgument,
                        "session.close.v1 requires a session-close payload",
                        false,
                    ));
                }
                let id = SessionId::new(command.session_id.clone()).map_err(|error| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::InvalidArgument,
                        error.to_string(),
                        false,
                    )
                })?;
                let mut sessions = self.sessions.lock().map_err(|_| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::CapacityUnavailable,
                        "session registry lock is poisoned",
                        true,
                    )
                })?;
                let mut session = sessions.remove(&id).ok_or_else(|| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::SessionClosed,
                        "session does not exist or is already closed",
                        false,
                    )
                })?;
                session.transition(SessionState::Closing).map_err(|error| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::SessionClosed,
                        error.to_string(),
                        false,
                    )
                })?;
                session.transition(SessionState::Closed).map_err(|error| {
                    DispatchError::failed(
                        machina_command_model::CanonicalErrorCode::SessionClosed,
                        error.to_string(),
                        false,
                    )
                })?;
                Ok("session closed".to_owned())
            }
            _ => Err(DispatchError::unsupported(format!("{:?}", command.kind))),
        }
    }
}

pub struct NativeEngine {
    inner: LifecycleEngine,
}

impl NativeEngine {
    pub fn new(build: impl Into<String>) -> Self {
        Self {
            inner: LifecycleEngine::new(EngineKind::Native, build),
        }
    }
}

impl EngineAdapter for NativeEngine {
    fn kind(&self) -> EngineKind {
        self.inner.kind
    }

    fn capabilities(&self) -> &CapabilitySnapshot {
        &self.inner.snapshot
    }

    fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<String, DispatchError> {
        self.inner.execute(command, context)
    }
}

pub struct ChromiumEngine {
    inner: LifecycleEngine,
}

impl ChromiumEngine {
    pub fn new(build: impl Into<String>) -> Self {
        Self {
            inner: LifecycleEngine::new(EngineKind::Chromium, build),
        }
    }
}

impl EngineAdapter for ChromiumEngine {
    fn kind(&self) -> EngineKind {
        self.inner.kind
    }

    fn capabilities(&self) -> &CapabilitySnapshot {
        &self.inner.snapshot
    }

    fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<String, DispatchError> {
        self.inner.execute(command, context)
    }
}

#[cfg(test)]
mod tests {
    use super::{ChromiumEngine, NativeEngine};
    use machina_command_bus::{CommandBus, CommandContext, FallbackPolicy};
    use machina_command_model::{
        CommandEnvelope, CommandKind, CommandMetadata, CommandPayload, SessionClosePayload,
        SessionCreatePayload,
    };
    use std::time::Duration;

    fn command(kind: CommandKind, session_id: &str) -> CommandEnvelope {
        let payload = match kind {
            CommandKind::SessionCreateV1 => CommandPayload::SessionCreate(SessionCreatePayload {
                engine_policy: "prefer-native".to_owned(),
                fidelity_profile: "agent".to_owned(),
            }),
            CommandKind::SessionCloseV1 => {
                CommandPayload::SessionClose(SessionClosePayload { reason: None })
            }
            _ => CommandPayload::SessionClose(SessionClosePayload { reason: None }),
        };
        CommandEnvelope {
            command_id: format!("{session_id}-command"),
            session_id: session_id.to_owned(),
            context_id: None,
            page_id: None,
            kind,
            payload,
            idempotency_key: None,
            deadline_ms: 5_000,
            required_capabilities: vec![match kind {
                CommandKind::SessionCreateV1 => "session.create.v1".to_owned(),
                CommandKind::SessionCloseV1 => "session.close.v1".to_owned(),
                _ => "unsupported.v1".to_owned(),
            }],
            metadata: CommandMetadata {
                correlation_id: format!("{session_id}-correlation"),
                causation_id: None,
                client: "native-core-test".to_owned(),
            },
        }
    }

    #[test]
    fn shared_bus_executes_session_lifecycle_with_native_metadata() {
        let bus = CommandBus::new(
            NativeEngine::new("native-test"),
            ChromiumEngine::new("chromium-test"),
            FallbackPolicy::PreferNative,
        );
        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        let created = bus
            .execute(
                &command(CommandKind::SessionCreateV1, "session-1"),
                &context,
            )
            .expect("session creation should succeed");
        assert_eq!(created.result.as_deref(), Some("session ready"));
        assert_eq!(
            created.execution.engine,
            machina_command_model::EngineKind::Native
        );

        let closed = bus
            .execute(&command(CommandKind::SessionCloseV1, "session-1"), &context)
            .expect("session close should succeed");
        assert_eq!(closed.result.as_deref(), Some("session closed"));
    }

    #[test]
    fn unsupported_navigation_is_explicit() {
        let bus = CommandBus::new(
            NativeEngine::new("native-test"),
            ChromiumEngine::new("chromium-test"),
            FallbackPolicy::PreferNative,
        );
        let mut unsupported = command(CommandKind::SessionCreateV1, "session-1");
        unsupported.kind = CommandKind::NavigationGotoV1;
        unsupported.required_capabilities = vec!["navigation.goto.v1".to_owned()];
        let error = bus
            .execute(
                &unsupported,
                &CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1)),
            )
            .expect_err("navigation is not implemented in this slice");
        assert_eq!(
            error.code,
            machina_command_model::CanonicalErrorCode::UnsupportedCapability
        );
    }
}
