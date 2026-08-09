//! Initial HTTP adapter contract.
//!
//! This crate maps a wire request to the single command bus. It does not
//! implement browser semantics or authorization bypasses.

use machina_command_bus::{CommandBus, CommandContext, DispatchError, EngineAdapter};
use machina_command_model::{CanonicalError, CommandEnvelope, CommandOutcome};
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub command: CommandEnvelope,
    pub correlation_id: String,
    pub timeout: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub outcome: Option<CommandOutcome>,
    pub error: Option<CanonicalError>,
}

pub struct HttpCommandAdapter<Native, Chromium> {
    bus: CommandBus<Native, Chromium>,
}

impl<Native, Chromium> HttpCommandAdapter<Native, Chromium>
where
    Native: EngineAdapter,
    Chromium: EngineAdapter,
{
    pub fn new(bus: CommandBus<Native, Chromium>) -> Self {
        Self { bus }
    }

    pub fn handle(&self, request: HttpRequest) -> HttpResponse {
        if request.method != "POST" || request.path != "/v1/commands" {
            return HttpResponse {
                status: 404,
                outcome: None,
                error: None,
            };
        }
        let context = CommandContext::with_timeout(request.correlation_id.clone(), request.timeout);
        match self.bus.execute(&request.command, &context) {
            Ok(outcome) => HttpResponse {
                status: 200,
                outcome: Some(outcome),
                error: None,
            },
            Err(error) => {
                let status = match error.code {
                    machina_command_model::CanonicalErrorCode::Unauthenticated => 401,
                    machina_command_model::CanonicalErrorCode::PermissionDenied
                    | machina_command_model::CanonicalErrorCode::PolicyDenied => 403,
                    machina_command_model::CanonicalErrorCode::QuotaExceeded
                    | machina_command_model::CanonicalErrorCode::RateLimited => 429,
                    machina_command_model::CanonicalErrorCode::CapacityUnavailable
                    | machina_command_model::CanonicalErrorCode::WorkerLost => 503,
                    machina_command_model::CanonicalErrorCode::UnsupportedCapability
                    | machina_command_model::CanonicalErrorCode::RendererRequired => 501,
                    _ => 400,
                };
                HttpResponse {
                    status,
                    outcome: None,
                    error: Some(canonical_error(&error, &request)),
                }
            }
        }
    }
}

fn canonical_error(error: &DispatchError, request: &HttpRequest) -> CanonicalError {
    CanonicalError {
        code: error.code,
        category: "protocol.http".to_owned(),
        message: error.message.clone(),
        retryable: error.retryable,
        retry_after_ms: None,
        engine: None,
        capability: error.capability.clone(),
        command_id: request.command.command_id.clone(),
        correlation_id: request.correlation_id.clone(),
        details: serde_json::json!({}),
        cause_code: None,
        documentation_ref: format!("errors/{:?}", error.code),
    }
}

#[cfg(test)]
mod tests {
    use super::{HttpCommandAdapter, HttpRequest};
    use machina_chromium_adapter::{ChromiumAdapter, ChromiumTransport, UnavailableChromium};
    use machina_command_bus::{CommandBus, CommandContext, DispatchError, FallbackPolicy};
    use machina_command_model::{
        CommandEnvelope, CommandKind, CommandMetadata, CommandPayload, SessionCreatePayload,
    };
    use machina_native_core::NativeEngine;
    use std::time::Duration;

    struct Runtime;
    impl ChromiumTransport for Runtime {
        fn execute(
            &self,
            _command: &CommandEnvelope,
            _context: &CommandContext,
        ) -> Result<String, DispatchError> {
            Ok("ok".to_owned())
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
                engine_policy: "prefer-compatible".to_owned(),
                fidelity_profile: "agent".to_owned(),
            }),
            idempotency_key: None,
            deadline_ms: 1000,
            required_capabilities: vec!["session.create.v1".to_owned()],
            metadata: CommandMetadata {
                correlation_id: "correlation-1".to_owned(),
                causation_id: None,
                client: "http-test".to_owned(),
            },
        }
    }

    #[test]
    fn maps_supported_command_and_not_found_path() {
        let mut chromium = ChromiumAdapter::new(Runtime, "chromium-test");
        chromium.register_capability("session.create.v1");
        let adapter = HttpCommandAdapter::new(CommandBus::new(
            NativeEngine::new("native-test"),
            chromium,
            FallbackPolicy::PreferCompatible,
        ));
        let response = adapter.handle(HttpRequest {
            method: "POST".to_owned(),
            path: "/v1/commands".to_owned(),
            command: command(),
            correlation_id: "correlation-1".to_owned(),
            timeout: Duration::from_secs(1),
        });
        assert_eq!(response.status, 200);
        assert_eq!(
            response.outcome.and_then(|outcome| outcome.result),
            Some("ok".to_owned())
        );
        let not_found = adapter.handle(HttpRequest {
            method: "GET".to_owned(),
            path: "/v1/commands".to_owned(),
            command: command(),
            correlation_id: "correlation-1".to_owned(),
            timeout: Duration::from_secs(1),
        });
        assert_eq!(not_found.status, 404);
    }

    #[test]
    fn maps_unavailable_runtime_to_explicit_501() {
        let mut chromium = ChromiumAdapter::new(UnavailableChromium, "chromium-test");
        chromium.register_capability("session.create.v1");
        let adapter = HttpCommandAdapter::new(CommandBus::new(
            NativeEngine::new("native-test"),
            chromium,
            FallbackPolicy::ChromiumOnly,
        ));
        let response = adapter.handle(HttpRequest {
            method: "POST".to_owned(),
            path: "/v1/commands".to_owned(),
            command: command(),
            correlation_id: "correlation-1".to_owned(),
            timeout: Duration::from_secs(1),
        });
        assert_eq!(response.status, 501);
        assert_eq!(
            response.error.expect("error").code,
            machina_command_model::CanonicalErrorCode::RendererRequired
        );
    }
}
