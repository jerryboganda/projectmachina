//! Chromium compatibility adapter behind the canonical command bus.
//!
//! The adapter translates to an injected transport. It never reports a
//! command as successful when the external Chromium runtime is unavailable.

use machina_capability::CapabilitySnapshot;
use machina_command_bus::{CommandContext, DispatchError, EngineAdapter};
use machina_command_model::{CapabilityStatus, CommandEnvelope, EngineKind};

pub trait ChromiumTransport {
    fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<String, DispatchError>;
}

#[derive(Clone, Debug)]
pub struct ChromiumAdapter<T> {
    transport: T,
    snapshot: CapabilitySnapshot,
}

impl<T> ChromiumAdapter<T> {
    pub fn new(transport: T, build: impl Into<String>) -> Self {
        Self {
            transport,
            snapshot: CapabilitySnapshot::new(EngineKind::Chromium, build),
        }
    }

    pub fn register_capability(
        &mut self,
        capability_id: impl Into<String>,
    ) -> Option<CapabilityStatus> {
        self.snapshot
            .register(capability_id, CapabilityStatus::Chromium)
    }
}

impl<T> EngineAdapter for ChromiumAdapter<T>
where
    T: ChromiumTransport,
{
    fn kind(&self) -> EngineKind {
        EngineKind::Chromium
    }

    fn capabilities(&self) -> &CapabilitySnapshot {
        &self.snapshot
    }

    fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<String, DispatchError> {
        self.transport.execute(command, context)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableChromium;

impl ChromiumTransport for UnavailableChromium {
    fn execute(
        &self,
        _command: &CommandEnvelope,
        _context: &CommandContext,
    ) -> Result<String, DispatchError> {
        Err(DispatchError::failed(
            machina_command_model::CanonicalErrorCode::RendererRequired,
            "Chromium compatibility runtime is unavailable",
            true,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{ChromiumAdapter, ChromiumTransport, UnavailableChromium};
    use machina_command_bus::{
        CommandBus, CommandContext, DispatchError, EngineAdapter, FallbackPolicy,
    };
    use machina_command_model::{
        CapabilityStatus, CommandEnvelope, CommandKind, CommandMetadata, CommandPayload,
        EngineKind, SessionCreatePayload,
    };
    use std::time::Duration;

    #[derive(Clone, Copy)]
    struct FakeTransport;

    impl ChromiumTransport for FakeTransport {
        fn execute(
            &self,
            _command: &CommandEnvelope,
            _context: &CommandContext,
        ) -> Result<String, DispatchError> {
            Ok("chromium-result".to_owned())
        }
    }

    fn command() -> CommandEnvelope {
        CommandEnvelope {
            command_id: "command-1".to_owned(),
            session_id: "session-1".to_owned(),
            context_id: None,
            page_id: None,
            kind: CommandKind::SessionCreateV1,
            payload: CommandPayload::SessionCreate(SessionCreatePayload {
                engine_policy: "chromium-only".to_owned(),
                fidelity_profile: "agent".to_owned(),
            }),
            idempotency_key: None,
            deadline_ms: 1000,
            required_capabilities: vec!["session.create.v1".to_owned()],
            metadata: CommandMetadata {
                correlation_id: "correlation-1".to_owned(),
                causation_id: None,
                client: "chromium-test".to_owned(),
            },
        }
    }

    #[test]
    fn reports_chromium_engine_and_capability_snapshot() {
        let mut adapter = ChromiumAdapter::new(FakeTransport, "chromium-test");
        adapter.register_capability("session.create.v1");
        assert_eq!(adapter.kind(), EngineKind::Chromium);
        assert_eq!(
            adapter.capabilities().status("session.create.v1"),
            Some(CapabilityStatus::Chromium)
        );
        let unavailable = ChromiumAdapter::new(UnavailableChromium, "unavailable");
        let bus = CommandBus::new(unavailable, adapter, FallbackPolicy::ChromiumOnly);
        let outcome = bus
            .execute(
                &command(),
                &CommandContext::with_timeout("correlation-1", Duration::from_secs(1)),
            )
            .expect("fake Chromium transport");
        assert_eq!(outcome.execution.engine, EngineKind::Chromium);
        assert_eq!(outcome.result.as_deref(), Some("chromium-result"));
    }

    #[test]
    fn unavailable_runtime_is_typed_failure() {
        let mut adapter = ChromiumAdapter::new(UnavailableChromium, "chromium-test");
        adapter.register_capability("session.create.v1");
        let result = adapter.execute(
            &command(),
            &CommandContext::with_timeout("correlation-1", Duration::from_secs(1)),
        );
        assert_eq!(
            result.expect_err("runtime must be unavailable").code,
            machina_command_model::CanonicalErrorCode::RendererRequired
        );
    }
}
