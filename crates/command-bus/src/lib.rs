//! The only internal route from public adapters to browser behavior.
//!
//! The bus owns routing, deadlines, cancellation, explicit capability checks,
//! and engine metadata. Engine adapters own browser semantics; protocol
//! adapters must not call them directly.

use std::fmt::{Display, Formatter};
use std::time::{Duration, Instant};

use machina_capability::CapabilitySnapshot;
use machina_command_model::{
    CanonicalError, CanonicalErrorCode, CommandEnvelope, CommandOutcome, EngineExecution,
    EngineKind, OutcomeStatus,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackPolicy {
    NativeOnly,
    PreferNative,
    PreferCompatible,
    ChromiumOnly,
}

#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
pub struct CommandContext {
    pub correlation_id: String,
    pub deadline: Instant,
    pub cancellation: CancellationToken,
}

impl CommandContext {
    pub fn with_timeout(correlation_id: impl Into<String>, timeout: Duration) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            deadline: Instant::now() + timeout,
            cancellation: CancellationToken::new(),
        }
    }

    pub fn is_cancelled_or_expired(&self) -> Option<CanonicalErrorCode> {
        if self.cancellation.is_cancelled() {
            return Some(CanonicalErrorCode::CommandCancelled);
        }
        if Instant::now() >= self.deadline {
            return Some(CanonicalErrorCode::DeadlineExceeded);
        }
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchError {
    pub code: CanonicalErrorCode,
    pub message: String,
    pub retryable: bool,
    pub capability: Option<String>,
}

impl DispatchError {
    pub fn unsupported(capability: impl Into<String>) -> Self {
        let capability = capability.into();
        Self {
            code: CanonicalErrorCode::UnsupportedCapability,
            message: format!("capability is not supported: {capability}"),
            retryable: false,
            capability: Some(capability),
        }
    }

    pub fn failed(code: CanonicalErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
            capability: None,
        }
    }
}

impl Display for DispatchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DispatchError {}

pub trait EngineAdapter {
    fn kind(&self) -> EngineKind;
    fn capabilities(&self) -> &CapabilitySnapshot;
    fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<String, DispatchError>;
}

pub struct CommandBus<Native, Chromium> {
    native: Native,
    chromium: Chromium,
    policy: FallbackPolicy,
}

impl<Native, Chromium> CommandBus<Native, Chromium>
where
    Native: EngineAdapter,
    Chromium: EngineAdapter,
{
    pub fn new(native: Native, chromium: Chromium, policy: FallbackPolicy) -> Self {
        Self {
            native,
            chromium,
            policy,
        }
    }

    pub fn policy(&self) -> FallbackPolicy {
        self.policy
    }

    pub fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<CommandOutcome, DispatchError> {
        if let Some(code) = context.is_cancelled_or_expired() {
            return Err(DispatchError::failed(
                code,
                code_name(code),
                code == CanonicalErrorCode::DeadlineExceeded,
            ));
        }

        let (adapter, fallback_used, fallback_reason) =
            self.select_adapter(&command.required_capabilities)?;
        let engine = adapter.kind();
        match adapter.execute(command, context) {
            Ok(result) => Ok(CommandOutcome {
                command_id: command.command_id.clone(),
                attempt: 1,
                status: OutcomeStatus::Succeeded,
                result: Some(result),
                error: None,
                execution: EngineExecution {
                    requested_engine_policy: policy_name(self.policy).to_owned(),
                    engine,
                    engine_version: adapter.capabilities().engine_build.clone(),
                    capability_snapshot: snapshot_name(adapter.capabilities()),
                    fallback_used,
                    fallback_reason,
                    migration_id: None,
                },
                trace_ref: None,
            }),
            Err(error) => Err(error),
        }
    }

    fn select_adapter(
        &self,
        required_capabilities: &[String],
    ) -> Result<(&dyn EngineAdapter, bool, Option<String>), DispatchError> {
        let native_ready = self
            .native
            .capabilities()
            .supports_all(required_capabilities.iter());
        let chromium_ready = self
            .chromium
            .capabilities()
            .supports_all(required_capabilities.iter());

        match self.policy {
            FallbackPolicy::NativeOnly if native_ready => Ok((&self.native, false, None)),
            FallbackPolicy::ChromiumOnly if chromium_ready => Ok((&self.chromium, false, None)),
            FallbackPolicy::PreferNative if native_ready => Ok((&self.native, false, None)),
            FallbackPolicy::PreferCompatible if chromium_ready => Ok((&self.chromium, false, None)),
            FallbackPolicy::PreferNative if chromium_ready => Ok((
                &self.chromium,
                true,
                Some("native_capability_miss".to_owned()),
            )),
            FallbackPolicy::PreferCompatible if native_ready => Ok((
                &self.native,
                true,
                Some("chromium_capability_miss".to_owned()),
            )),
            FallbackPolicy::NativeOnly => Err(DispatchError::failed(
                CanonicalErrorCode::UnsupportedCapability,
                "native-only policy cannot satisfy required capabilities",
                false,
            )),
            FallbackPolicy::ChromiumOnly => Err(DispatchError::failed(
                CanonicalErrorCode::UnsupportedCapability,
                "chromium-only policy cannot satisfy required capabilities",
                false,
            )),
            _ => Err(DispatchError::failed(
                CanonicalErrorCode::UnsupportedCapability,
                "no configured engine can satisfy required capabilities",
                false,
            )),
        }
    }
}

fn policy_name(policy: FallbackPolicy) -> &'static str {
    match policy {
        FallbackPolicy::NativeOnly => "native-only",
        FallbackPolicy::PreferNative => "prefer-native",
        FallbackPolicy::PreferCompatible => "prefer-compatible",
        FallbackPolicy::ChromiumOnly => "chromium-only",
    }
}

fn snapshot_name(snapshot: &CapabilitySnapshot) -> String {
    snapshot
        .iter()
        .map(|(id, status)| format!("{id}:{status:?}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn code_name(code: CanonicalErrorCode) -> &'static str {
    match code {
        CanonicalErrorCode::CommandCancelled => "COMMAND_CANCELLED",
        CanonicalErrorCode::DeadlineExceeded => "DEADLINE_EXCEEDED",
        CanonicalErrorCode::UnsupportedCapability => "UNSUPPORTED_CAPABILITY",
        _ => "COMMAND_DISPATCH_FAILED",
    }
}

pub fn to_canonical_error(
    error: &DispatchError,
    command: &CommandEnvelope,
    context: &CommandContext,
    engine: Option<EngineKind>,
) -> CanonicalError {
    CanonicalError {
        code: error.code,
        category: "dispatch".to_owned(),
        message: error.message.clone(),
        retryable: error.retryable,
        retry_after_ms: None,
        engine,
        capability: error.capability.clone(),
        command_id: command.command_id.clone(),
        correlation_id: context.correlation_id.clone(),
        details: serde_json::json!({}),
        cause_code: None,
        documentation_ref: format!("errors/{}", code_name(error.code)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CancellationToken, CommandBus, CommandContext, DispatchError, EngineAdapter, FallbackPolicy,
    };
    use machina_capability::CapabilitySnapshot;
    use machina_command_model::{
        CapabilityStatus, CommandEnvelope, CommandKind, CommandMetadata, CommandPayload,
        EngineKind, SessionCreatePayload,
    };
    use std::time::Duration;

    struct TestEngine {
        snapshot: CapabilitySnapshot,
        result: Result<String, DispatchError>,
    }

    impl EngineAdapter for TestEngine {
        fn kind(&self) -> EngineKind {
            self.snapshot.engine
        }

        fn capabilities(&self) -> &CapabilitySnapshot {
            &self.snapshot
        }

        fn execute(
            &self,
            _command: &CommandEnvelope,
            _context: &CommandContext,
        ) -> Result<String, DispatchError> {
            self.result.clone()
        }
    }

    fn command(capability: &str) -> CommandEnvelope {
        CommandEnvelope {
            command_id: "command-1".to_owned(),
            session_id: "session-1".to_owned(),
            context_id: None,
            page_id: None,
            kind: CommandKind::SessionCreateV1,
            payload: CommandPayload::SessionCreate(SessionCreatePayload {
                engine_policy: "prefer-native".to_owned(),
                fidelity_profile: "agent".to_owned(),
            }),
            idempotency_key: None,
            deadline_ms: 5000,
            required_capabilities: vec![capability.to_owned()],
            metadata: CommandMetadata {
                correlation_id: "correlation-1".to_owned(),
                causation_id: None,
                client: "test".to_owned(),
            },
        }
    }

    fn engine(
        kind: EngineKind,
        capability: &str,
        status: CapabilityStatus,
        result: &str,
    ) -> TestEngine {
        let mut snapshot = CapabilitySnapshot::new(kind, format!("{kind:?}-test"));
        snapshot.register(capability, status);
        TestEngine {
            snapshot,
            result: Ok(result.to_owned()),
        }
    }

    #[test]
    fn routes_native_first_and_reports_engine_metadata() {
        let bus = CommandBus::new(
            engine(
                EngineKind::Native,
                "session.create.v1",
                CapabilityStatus::Native,
                "native",
            ),
            engine(
                EngineKind::Chromium,
                "session.create.v1",
                CapabilityStatus::Chromium,
                "chromium",
            ),
            FallbackPolicy::PreferNative,
        );
        let result = bus
            .execute(
                &command("session.create.v1"),
                &CommandContext::with_timeout("correlation-1", Duration::from_secs(1)),
            )
            .expect("test engine should succeed");
        assert_eq!(result.result.as_deref(), Some("native"));
        assert_eq!(result.execution.engine, EngineKind::Native);
        assert!(!result.execution.fallback_used);
    }

    #[test]
    fn falls_back_explicitly_when_native_lacks_capability() {
        let bus = CommandBus::new(
            engine(
                EngineKind::Native,
                "visual.screenshot.v1",
                CapabilityStatus::Unsupported,
                "native",
            ),
            engine(
                EngineKind::Chromium,
                "visual.screenshot.v1",
                CapabilityStatus::Chromium,
                "chromium",
            ),
            FallbackPolicy::PreferNative,
        );
        let result = bus
            .execute(
                &command("visual.screenshot.v1"),
                &CommandContext::with_timeout("correlation-1", Duration::from_secs(1)),
            )
            .expect("Chromium fallback should be eligible");
        assert_eq!(result.execution.engine, EngineKind::Chromium);
        assert!(result.execution.fallback_used);
        assert_eq!(
            result.execution.fallback_reason.as_deref(),
            Some("native_capability_miss")
        );
    }

    #[test]
    fn cancellation_and_native_only_miss_are_typed_failures() {
        let bus = CommandBus::new(
            engine(
                EngineKind::Native,
                "session.create.v1",
                CapabilityStatus::Unsupported,
                "native",
            ),
            engine(
                EngineKind::Chromium,
                "session.create.v1",
                CapabilityStatus::Chromium,
                "chromium",
            ),
            FallbackPolicy::NativeOnly,
        );
        let context = CommandContext::with_timeout("correlation-1", Duration::from_secs(1));
        let token: CancellationToken = context.cancellation.clone();
        token.cancel();
        let error = bus
            .execute(&command("session.create.v1"), &context)
            .expect_err("cancelled command must fail");
        assert_eq!(
            error.code,
            machina_command_model::CanonicalErrorCode::CommandCancelled
        );

        let error = bus
            .execute(
                &command("session.create.v1"),
                &CommandContext::with_timeout("correlation-1", Duration::from_secs(1)),
            )
            .expect_err("native-only capability miss must fail");
        assert_eq!(
            error.code,
            machina_command_model::CanonicalErrorCode::UnsupportedCapability
        );
    }
}
