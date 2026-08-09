//! The only internal route from public adapters to browser behavior.
//!
//! The bus owns routing, deadlines, cancellation, explicit capability checks,
//! and engine metadata. Engine adapters own browser semantics; protocol
//! adapters must not call them directly.

use std::fmt::{Display, Formatter};
use std::time::{Duration, Instant};

use machina_capability::{CapabilityRegistry, CapabilitySnapshot};
use machina_command_model::{
    CanonicalError, CanonicalErrorCode, CapabilityStatus, CommandEnvelope, CommandOutcome,
    EngineExecution, EngineKind, OutcomeStatus,
};
use machina_telemetry::{DataClassification, SessionTrace};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub type SharedTrace = Arc<Mutex<SessionTrace>>;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FallbackPolicy {
    #[serde(rename = "native-only")]
    NativeOnly,
    #[serde(rename = "prefer-native")]
    PreferNative,
    #[serde(rename = "prefer-compatible")]
    PreferCompatible,
    #[serde(rename = "chromium-only")]
    ChromiumOnly,
}

impl FallbackPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeOnly => "native-only",
            Self::PreferNative => "prefer-native",
            Self::PreferCompatible => "prefer-compatible",
            Self::ChromiumOnly => "chromium-only",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RoutingReason {
    #[serde(rename = "native_policy")]
    NativePolicy,
    #[serde(rename = "chromium_policy")]
    ChromiumPolicy,
    #[serde(rename = "native_capabilities")]
    NativeCapabilities,
    #[serde(rename = "chromium_capabilities")]
    ChromiumCapabilities,
    #[serde(rename = "native_capability_miss")]
    NativeCapabilityMiss,
    #[serde(rename = "chromium_capability_miss")]
    ChromiumCapabilityMiss,
    #[serde(rename = "no_compatible_engine")]
    NoCompatibleEngine,
}

impl RoutingReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativePolicy => "native_policy",
            Self::ChromiumPolicy => "chromium_policy",
            Self::NativeCapabilities => "native_capabilities",
            Self::ChromiumCapabilities => "chromium_capabilities",
            Self::NativeCapabilityMiss => "native_capability_miss",
            Self::ChromiumCapabilityMiss => "chromium_capability_miss",
            Self::NoCompatibleEngine => "no_compatible_engine",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoutingDecisionRecord {
    pub policy: FallbackPolicy,
    pub selected_engine: Option<EngineKind>,
    pub reason: RoutingReason,
    pub engine_version: Option<String>,
    pub capability_version: Option<String>,
    pub capability_snapshot: Option<String>,
    pub available_evidence: Vec<CapabilityEvidence>,
    pub fallback_used: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityEvidence {
    pub engine: EngineKind,
    pub engine_version: String,
    pub capability_version: String,
    pub capability_snapshot: String,
}

#[derive(Debug)]
pub struct DispatchResult {
    pub outcome: CommandOutcome,
    pub decision: RoutingDecisionRecord,
}

#[derive(Debug)]
pub struct DispatchFailure {
    pub error: DispatchError,
    pub decision: RoutingDecisionRecord,
}

impl Display for DispatchFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for DispatchFailure {}

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
    pub trace_ref: Option<String>,
    pub trace: Option<SharedTrace>,
}

impl CommandContext {
    pub fn with_timeout(correlation_id: impl Into<String>, timeout: Duration) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            deadline: Instant::now() + timeout,
            cancellation: CancellationToken::new(),
            trace_ref: None,
            trace: None,
        }
    }

    pub fn with_trace(
        correlation_id: impl Into<String>,
        timeout: Duration,
        trace_ref: impl Into<String>,
        trace: SharedTrace,
    ) -> Self {
        let mut context = Self::with_timeout(correlation_id, timeout);
        context.trace_ref = Some(trace_ref.into());
        context.trace = Some(trace);
        context
    }

    pub fn record_trace(
        &self,
        event_type: &str,
        redacted_message: &str,
    ) -> Result<(), DispatchError> {
        let Some(trace) = &self.trace else {
            return Ok(());
        };
        let mut trace = trace.lock().map_err(|_| {
            DispatchError::failed(
                CanonicalErrorCode::CapacityUnavailable,
                "trace recorder is unavailable",
                true,
            )
        })?;
        let prefix = self.trace_ref.as_deref().unwrap_or("trace");
        let event_id = format!("{prefix}:{}:{}", event_type, trace.len() + 1);
        trace
            .append(
                event_id,
                event_type,
                timestamp_now(),
                DataClassification::Restricted,
                redacted_message,
            )
            .map(|_| ())
            .map_err(|_| {
                DispatchError::failed(
                    CanonicalErrorCode::CapacityUnavailable,
                    "trace recorder rejected event",
                    true,
                )
            })
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

    pub fn capability_registry(&self) -> CapabilityRegistry {
        let mut registry = CapabilityRegistry::default();
        registry.upsert(self.native.capabilities().clone());
        registry.upsert(self.chromium.capabilities().clone());
        registry
    }

    pub fn decide(&self, required_capabilities: &[String]) -> RoutingDecisionRecord {
        let native_ready = self
            .native
            .capabilities()
            .supports_all(required_capabilities.iter());
        let chromium_ready = self
            .chromium
            .capabilities()
            .supports_all(required_capabilities.iter());

        let (selected_engine, fallback_used, reason) = match self.policy {
            FallbackPolicy::NativeOnly if native_ready => {
                (Some(EngineKind::Native), false, RoutingReason::NativePolicy)
            }
            FallbackPolicy::ChromiumOnly if chromium_ready => (
                Some(EngineKind::Chromium),
                false,
                RoutingReason::ChromiumPolicy,
            ),
            FallbackPolicy::PreferNative if native_ready => (
                Some(EngineKind::Native),
                false,
                RoutingReason::NativeCapabilities,
            ),
            FallbackPolicy::PreferCompatible if chromium_ready => (
                Some(EngineKind::Chromium),
                false,
                RoutingReason::ChromiumCapabilities,
            ),
            FallbackPolicy::PreferNative if chromium_ready => (
                Some(EngineKind::Chromium),
                true,
                RoutingReason::NativeCapabilityMiss,
            ),
            FallbackPolicy::PreferCompatible if native_ready => (
                Some(EngineKind::Native),
                true,
                RoutingReason::ChromiumCapabilityMiss,
            ),
            _ => (None, false, RoutingReason::NoCompatibleEngine),
        };
        let snapshot = selected_engine.map(|engine| match engine {
            EngineKind::Native => self.native.capabilities(),
            EngineKind::Chromium => self.chromium.capabilities(),
        });
        let available_evidence = [self.native.capabilities(), self.chromium.capabilities()]
            .into_iter()
            .map(capability_evidence)
            .collect();
        RoutingDecisionRecord {
            policy: self.policy,
            selected_engine,
            reason,
            engine_version: snapshot.map(|value| value.engine_build.clone()),
            capability_version: snapshot.map(|value| value.version().to_owned()),
            capability_snapshot: snapshot.map(snapshot_name),
            available_evidence,
            fallback_used,
        }
    }

    pub fn execute(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<CommandOutcome, DispatchError> {
        self.execute_with_decision(command, context)
            .map_err(|failure| failure.error)
            .map(|result| result.outcome)
    }

    pub fn execute_with_decision(
        &self,
        command: &CommandEnvelope,
        context: &CommandContext,
    ) -> Result<DispatchResult, Box<DispatchFailure>> {
        let decision = self.decide(&command.required_capabilities);
        if let Some(code) = context.is_cancelled_or_expired() {
            let error = DispatchError::failed(
                code,
                code_name(code),
                code == CanonicalErrorCode::DeadlineExceeded,
            );
            if let Err(trace_error) = context.record_trace("command.cancelled.v1", code_name(code))
            {
                return Err(Box::new(DispatchFailure {
                    error: trace_error,
                    decision,
                }));
            }
            return Err(Box::new(DispatchFailure { error, decision }));
        }

        if let Err(error) = context.record_trace(
            "capability.decision.v1",
            &format!(
                "engine={:?};reason={}",
                decision.selected_engine,
                decision.reason.as_str()
            ),
        ) {
            return Err(Box::new(DispatchFailure { error, decision }));
        }
        let adapter = match decision.selected_engine {
            Some(EngineKind::Native) => &self.native as &dyn EngineAdapter,
            Some(EngineKind::Chromium) => &self.chromium as &dyn EngineAdapter,
            None => {
                let error = DispatchError::failed(
                    CanonicalErrorCode::UnsupportedCapability,
                    "no configured engine can satisfy required capabilities",
                    false,
                );
                if let Err(trace_error) =
                    context.record_trace("routing.rejected.v1", "no_compatible_engine")
                {
                    return Err(Box::new(DispatchFailure {
                        error: trace_error,
                        decision,
                    }));
                }
                return Err(Box::new(DispatchFailure { error, decision }));
            }
        };
        let engine = adapter.kind();
        match adapter.execute(command, context) {
            Ok(result) => Ok(DispatchResult {
                outcome: {
                    if let Err(error) = context.record_trace("worker.outcome.v1", "verified") {
                        return Err(Box::new(DispatchFailure { error, decision }));
                    }
                    CommandOutcome {
                        command_id: command.command_id.clone(),
                        attempt: 1,
                        status: OutcomeStatus::Succeeded,
                        result: Some(result),
                        error: None,
                        execution: EngineExecution {
                            requested_engine_policy: self.policy.as_str().to_owned(),
                            engine,
                            engine_version: adapter.capabilities().engine_build.clone(),
                            capability_snapshot: snapshot_name(adapter.capabilities()),
                            fallback_used: decision.fallback_used,
                            fallback_reason: decision
                                .fallback_used
                                .then(|| decision.reason.as_str().to_owned()),
                            migration_id: None,
                        },
                        trace_ref: context.trace_ref.clone(),
                    }
                },
                decision,
            }),
            Err(error) => {
                if let Err(trace_error) = context.record_trace("command.error.v1", "adapter_failed")
                {
                    return Err(Box::new(DispatchFailure {
                        error: trace_error,
                        decision,
                    }));
                }
                Err(Box::new(DispatchFailure { error, decision }))
            }
        }
    }
}

fn capability_evidence(snapshot: &CapabilitySnapshot) -> CapabilityEvidence {
    CapabilityEvidence {
        engine: snapshot.engine,
        engine_version: snapshot.engine_build.clone(),
        capability_version: snapshot.version().to_owned(),
        capability_snapshot: snapshot_name(snapshot),
    }
}

fn snapshot_name(snapshot: &CapabilitySnapshot) -> String {
    let capabilities = snapshot
        .iter()
        .map(|(id, status)| format!("{id}:{}", capability_status_name(status)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}@{}:{}",
        snapshot.version(),
        snapshot.engine_build,
        capabilities
    )
}

fn timestamp_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

fn capability_status_name(status: CapabilityStatus) -> &'static str {
    match status {
        CapabilityStatus::Native => "native",
        CapabilityStatus::NativeLimited => "native-limited",
        CapabilityStatus::Hybrid => "hybrid",
        CapabilityStatus::Chromium => "chromium",
        CapabilityStatus::Experimental => "experimental",
        CapabilityStatus::Unsupported => "unsupported",
        CapabilityStatus::DisabledByPolicy => "disabled-by-policy",
    }
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
        CancellationToken, CommandBus, CommandContext, DispatchError, EngineAdapter,
        FallbackPolicy, RoutingReason,
    };
    use machina_capability::CapabilitySnapshot;
    use machina_command_model::{
        CapabilityStatus, CommandEnvelope, CommandKind, CommandMetadata, CommandPayload,
        EngineKind, SessionCreatePayload,
    };
    use machina_telemetry::{CorrelationContext, DataClassification, SessionTrace};
    use std::sync::{Arc, Mutex};
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
    fn emits_versioned_decision_and_matching_capability_registry() {
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
        let decision = bus.decide(&["session.create.v1".to_owned()]);
        assert_eq!(decision.policy, FallbackPolicy::PreferNative);
        assert_eq!(decision.selected_engine, Some(EngineKind::Native));
        assert_eq!(decision.reason, RoutingReason::NativeCapabilities);
        assert_eq!(
            decision.capability_version.as_deref(),
            Some(machina_capability::CAPABILITY_SNAPSHOT_VERSION)
        );
        assert_eq!(decision.engine_version.as_deref(), Some("Native-test"));
        let registry = bus.capability_registry();
        assert_eq!(registry.len(), 2);
        assert_eq!(
            registry
                .snapshot(EngineKind::Native)
                .and_then(|snapshot| snapshot.status("session.create.v1")),
            Some(CapabilityStatus::Native)
        );
        let wire = serde_json::to_value(&decision).expect("decision serializes");
        assert_eq!(wire["policy"], "prefer-native");
        assert_eq!(wire["reason"], "native_capabilities");
        assert_eq!(wire["capability_version"], "capability.v0");
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

    #[test]
    fn disabled_policy_and_no_capacity_never_assign_an_engine() {
        let mut native = engine(
            EngineKind::Native,
            "session.create.v1",
            CapabilityStatus::Native,
            "native",
        );
        native
            .snapshot
            .set_policy_status("session.create.v1", false)
            .expect("registered capability");
        let bus = CommandBus::new(
            native,
            engine(
                EngineKind::Chromium,
                "session.create.v1",
                CapabilityStatus::Unsupported,
                "chromium",
            ),
            FallbackPolicy::PreferCompatible,
        );
        let decision = bus.decide(&["session.create.v1".to_owned()]);
        assert_eq!(decision.selected_engine, None);
        assert_eq!(decision.reason, RoutingReason::NoCompatibleEngine);
        assert_eq!(decision.available_evidence.len(), 2);
        assert!(decision
            .available_evidence
            .iter()
            .all(|evidence| !evidence.capability_version.is_empty()));
        let failure = bus
            .execute_with_decision(
                &command("session.create.v1"),
                &CommandContext::with_timeout("correlation-1", Duration::from_secs(1)),
            )
            .expect_err("decision must accompany no-capacity failure");
        assert_eq!(failure.decision.reason, RoutingReason::NoCompatibleEngine);
        assert_eq!(
            bus.execute(
                &command("session.create.v1"),
                &CommandContext::with_timeout("correlation-1", Duration::from_secs(1))
            )
            .expect_err("no compatible engine")
            .code,
            machina_command_model::CanonicalErrorCode::UnsupportedCapability
        );
    }

    #[test]
    fn records_routing_and_worker_outcome_on_shared_trace() {
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
        let trace = Arc::new(Mutex::new(
            SessionTrace::new(
                CorrelationContext {
                    correlation_id: "correlation-1".to_owned(),
                    causation_id: None,
                    task_id: None,
                    command_id: Some("command-1".to_owned()),
                    session_id: Some("session-1".to_owned()),
                },
                8,
            )
            .expect("trace"),
        ));
        let context = super::CommandContext::with_trace(
            "correlation-1",
            Duration::from_secs(1),
            "trace-1",
            trace.clone(),
        );
        let result = bus
            .execute_with_decision(&command("session.create.v1"), &context)
            .expect("dispatch");
        assert_eq!(result.outcome.trace_ref.as_deref(), Some("trace-1"));
        let trace = trace.lock().expect("trace lock");
        let event_types = trace
            .events()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            event_types,
            vec!["capability.decision.v1", "worker.outcome.v1"]
        );
        assert!(trace
            .events()
            .all(|event| event.classification == DataClassification::Restricted));
    }
}
