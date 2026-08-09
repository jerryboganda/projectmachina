//! Engine adapter foundation shared by native and Chromium worker paths.
//!
//! `session.create.v1` / `session.close.v1` / `interaction.click.v1` are the
//! commands routed through the shared `CommandBus`/`EngineAdapter` boundary
//! in this slice; every other command kind returns a typed
//! unsupported-capability error, it is never treated as a successful no-op.
//!
//! `interaction.click.v1` is Native-only: it resolves `ClickPayload.selector`
//! against the session's live `machina_dom::Document` (via
//! `machina_selectors::query_selector_all`) and performs a synthetic click
//! (via `machina_events::perform_click`), mapping both crates' typed errors
//! onto the existing `CanonicalErrorCode` variants the M1/M2 contract
//! compatibility checklist identified for exactly this purpose — no new
//! error codes. The Chromium path deliberately does **not** register this
//! capability and explicitly rejects it if reached directly (bypassing the
//! bus): this crate's `EngineSession` document/registry are native-engine
//! bookkeeping, not a real Chromium render, so faking a Chromium click
//! against them would silently misreport engine provenance. See
//! `.agent-state/evidence/wire-interaction-click.md` for the full mapping
//! table and design decisions (ambiguity detection, malformed-selector
//! mapping, and the `interaction.click.result.v1` result envelope).
//!
//! In addition to the command-bus surface, this crate composes the
//! `EngineSession` facade: the per-session object that owns browsing-context
//! and page identities, their lifecycle/cancellation, and their bounded
//! resource counters, plus (new in this slice) the session's live
//! `machina_dom::Document` and `machina_events::EventTargetRegistry`.
//! `EngineSession` is a native engine composition type — it consumes only
//! canonical `machina-session`/`machina-command-model`/`machina-capability`/
//! `machina-dom`/`machina-selectors`/`machina-events` types and never a
//! protocol- or control-plane-layer type. Context/page create/close/cancel
//! and resource accounting are not yet reachable through a canonical
//! `CommandKind` (the command schema does not define
//! `context.create.v1`/`page.create.v1` etc. yet), so they are exposed as
//! direct Rust APIs on the engines, ready to be wired to those commands once
//! the schema defines them. `Document` population is similarly exposed as a
//! direct Rust API (`with_document_mut`) today — this task does not wire
//! `navigation.goto.v1` (M2-T09's job); it only makes sure the plumbing a
//! future navigation command needs to populate the document already exists.

use std::collections::BTreeMap;
use std::sync::Mutex;

use machina_capability::{CapabilityResponse, CapabilitySnapshot};
use machina_command_bus::{CancellationToken, CommandContext, DispatchError, EngineAdapter};
use machina_command_model::{
    CanonicalErrorCode, CapabilityStatus, CommandEnvelope, CommandKind, CommandPayload, EngineKind,
};
use machina_dom::Document;
use machina_events::{EventError, EventTargetRegistry, PostconditionState};
use machina_selectors::QueryError;
use machina_session::{
    ContextId, LifecycleState, PageId, PageResourceBudget, PageResourceKind, ResourceBudget,
    ResourceUsage, Session, SessionError, SessionId,
};

/// Per-session health, suitable for worker/scheduler polling: lifecycle
/// state, structural counters (contexts/pages), and the session's resource
/// budget/usage. Carries no protocol-layer type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionHealth {
    pub session_id: String,
    pub engine: EngineKind,
    pub state: LifecycleState,
    pub context_count: usize,
    pub page_count: usize,
    pub usage: ResourceUsage,
    pub budget: ResourceBudget,
}

/// Engine-level health/capability snapshot exposed to the worker/scheduler
/// layer: the engine's capability response plus the health of every session
/// it currently composes.
#[derive(Clone, Debug)]
pub struct EngineHealth {
    pub engine: EngineKind,
    pub engine_build: String,
    pub capability_snapshot: CapabilityResponse,
    pub sessions: Vec<SessionHealth>,
}

/// The canonical native engine composition unit: one active session's engine
/// identity plus the `machina_session::Session` it owns (which in turn owns
/// browsing-context and page identities, their lifecycle/cancellation, and
/// their bounded resource counters). Both `NativeEngine` and `ChromiumEngine`
/// compose sessions through this same facade so their contract shape never
/// diverges.
pub struct EngineSession {
    engine: EngineKind,
    session: Session,
    /// This session's live native DOM document. Owned here (not in
    /// `machina-session`, which documents itself as DOM-agnostic
    /// foundation layer, and never in `machina-dom`/`machina-events`
    /// themselves, which stay engine-agnostic leaf crates) because
    /// `EngineSession` is exactly the native-engine composition point that
    /// is supposed to own this kind of cross-crate wiring. Empty at
    /// creation; populated by whichever command kind first mutates a
    /// session's content (today: only this crate's own tests exercising
    /// `interaction.click.v1`; eventually M2-T09's `navigation.goto.v1`).
    document: Document,
    /// Listener/focus state for [`EngineSession::document`]. One registry
    /// per document, per `machina_events`'s documented contract.
    registry: EventTargetRegistry,
}

impl EngineSession {
    fn new(engine: EngineKind, id: SessionId, budget: ResourceBudget) -> Self {
        let document = Document::new();
        let registry = EventTargetRegistry::for_document(&document);
        Self {
            engine,
            session: Session::new(id, budget),
            document,
            registry,
        }
    }

    pub fn engine(&self) -> EngineKind {
        self.engine
    }

    pub fn id(&self) -> &SessionId {
        self.session.id()
    }

    pub fn state(&self) -> LifecycleState {
        self.session.state()
    }

    pub fn budget(&self) -> ResourceBudget {
        self.session.budget()
    }

    pub fn usage(&self) -> ResourceUsage {
        self.session.usage()
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.session.cancellation()
    }

    pub fn transition(&mut self, next: LifecycleState) -> Result<(), SessionError> {
        self.session.transition(next)
    }

    /// Cancel the session (and cascade to every context/page it owns)
    /// immediately, regardless of in-flight work.
    pub fn cancel(&mut self) -> Result<(), SessionError> {
        self.session.cancel()
    }

    pub fn context_count(&self) -> usize {
        self.session.context_count()
    }

    pub fn page_count(&self) -> usize {
        self.session
            .contexts()
            .map(|context| context.page_count())
            .sum()
    }

    pub fn open_context(&mut self, context_id: ContextId) -> Result<(), SessionError> {
        self.session.open_context(context_id).map(|_| ())
    }

    pub fn close_context(&mut self, context_id: &ContextId) -> Result<(), SessionError> {
        self.session.close_context(context_id)
    }

    pub fn open_page(
        &mut self,
        context_id: &ContextId,
        page_id: PageId,
        budget: PageResourceBudget,
    ) -> Result<(), SessionError> {
        self.session.open_page(context_id, page_id, budget)
    }

    pub fn close_page(
        &mut self,
        context_id: &ContextId,
        page_id: &PageId,
    ) -> Result<(), SessionError> {
        self.session.close_page(context_id, page_id)
    }

    pub fn account_page_resource(
        &mut self,
        context_id: &ContextId,
        page_id: &PageId,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), SessionError> {
        self.session
            .context_mut(context_id)
            .ok_or(SessionError::ContextNotFound)?
            .page_mut(page_id)
            .ok_or(SessionError::PageNotFound)?
            .account(kind, amount)
    }

    pub fn release_page_resource(
        &mut self,
        context_id: &ContextId,
        page_id: &PageId,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), SessionError> {
        self.session
            .context_mut(context_id)
            .ok_or(SessionError::ContextNotFound)?
            .page_mut(page_id)
            .ok_or(SessionError::PageNotFound)?
            .release(kind, amount)
    }

    fn health(&self) -> SessionHealth {
        SessionHealth {
            session_id: self.id().as_str().to_owned(),
            engine: self.engine,
            state: self.state(),
            context_count: self.context_count(),
            page_count: self.page_count(),
            usage: self.usage(),
            budget: self.budget(),
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// `interaction.click.v1`'s implementation: resolve `selector` against
    /// this session's live document, then perform a synthetic click on the
    /// resolved element. See this crate's module docs and
    /// `.agent-state/evidence/wire-interaction-click.md` for the exact
    /// error-code mapping and the ambiguity/result-envelope decisions.
    fn click(&mut self, selector: &str) -> Result<String, DispatchError> {
        let matches = machina_selectors::query_selector_all(&self.document, selector)
            .map_err(map_query_error)?;
        let target = match matches.elements.as_slice() {
            [] => {
                return Err(DispatchError::failed(
                    CanonicalErrorCode::ElementNotFound,
                    format!("no element matched selector {selector:?}"),
                    false,
                ));
            }
            [only] => only.node_handle(),
            multiple => {
                return Err(DispatchError::failed(
                    CanonicalErrorCode::ElementAmbiguous,
                    format!(
                        "selector {selector:?} matched {} elements; interaction.click.v1 requires exactly one match",
                        multiple.len()
                    ),
                    false,
                ));
            }
        };

        let outcome = machina_events::perform_click(&mut self.document, &mut self.registry, target)
            .map_err(map_event_error)?;

        match &outcome.postcondition {
            PostconditionState::Verified => Ok(click_result_envelope(&outcome)),
            PostconditionState::Failed(reason) => Err(DispatchError::failed(
                CanonicalErrorCode::ActionPostconditionFailed,
                reason.clone(),
                false,
            )),
        }
    }
}

/// Maps `machina_selectors::QueryError` onto `CanonicalErrorCode`. Decision
/// record (see `.agent-state/evidence/wire-interaction-click.md`):
/// `InvalidSelector` (malformed syntax), `UnsupportedFeature` (valid syntax,
/// out-of-scope construct), and `TooComplex`/`ContextNodeRequired` (bounded-
/// walk guards / a CSS-only call path that cannot actually produce this
/// variant) all map to `SELECTOR_INVALID`: from `interaction.click.v1`'s
/// caller's point of view every one of these means "the selector you gave me
/// cannot be used to resolve an element," which is exactly what
/// `SELECTOR_INVALID` denotes, and no finer-grained code exists in the
/// pinned `CanonicalErrorCode` enum. `DomError` (an internal invariant
/// failure — this crate always calls the query against its own session's
/// live document/handles, so this should be structurally unreachable) maps
/// to `ACTION_POSTCONDITION_FAILED`, the closest "we could not complete this
/// action" bucket, with the underlying detail preserved in the message.
fn map_query_error(error: QueryError) -> DispatchError {
    match error {
        QueryError::InvalidSelector { message, position } => DispatchError::failed(
            CanonicalErrorCode::SelectorInvalid,
            format!("invalid selector syntax at byte {position}: {message}"),
            false,
        ),
        QueryError::UnsupportedFeature { feature, position } => DispatchError::failed(
            CanonicalErrorCode::SelectorInvalid,
            format!(
                "selector uses an unsupported (out-of-scope) feature at byte {position}: {feature}"
            ),
            false,
        ),
        QueryError::TooComplex { limit } => DispatchError::failed(
            CanonicalErrorCode::SelectorInvalid,
            format!("selector exceeded a bounded-walk guard ({limit}); treated as unusable, not silently truncated"),
            false,
        ),
        QueryError::ContextNodeRequired => DispatchError::failed(
            CanonicalErrorCode::SelectorInvalid,
            "selector requires a context node, which interaction.click.v1 does not supply",
            false,
        ),
        QueryError::DomError(inner) => DispatchError::failed(
            CanonicalErrorCode::ActionPostconditionFailed,
            format!("internal dom error while resolving selector: {inner}"),
            false,
        ),
        other => DispatchError::failed(
            CanonicalErrorCode::SelectorInvalid,
            format!("selector could not be resolved: {other}"),
            false,
        ),
    }
}

/// Maps `machina_events::EventError` onto `CanonicalErrorCode`.
/// `NotInteractable` is the one case `interaction.click.v1` names explicitly
/// (`ELEMENT_NOT_INTERACTABLE`). `TargetNotFound` maps to
/// `ELEMENT_NOT_FOUND`: it can only happen if the element resolved by the
/// selector query stopped resolving in the brief window before
/// `perform_click` re-validated it, which is the same caller-facing meaning
/// as "no element matched." `WrongDocument`/`Dom` are internal-invariant
/// failures (this crate always passes its own session's document/registry
/// and a handle just resolved from that same document) and map to
/// `ACTION_POSTCONDITION_FAILED`, the closest "could not complete this
/// action" bucket.
fn map_event_error(error: EventError) -> DispatchError {
    match error {
        EventError::NotInteractable => DispatchError::failed(
            CanonicalErrorCode::ElementNotInteractable,
            error.to_string(),
            false,
        ),
        EventError::TargetNotFound => DispatchError::failed(
            CanonicalErrorCode::ElementNotFound,
            error.to_string(),
            false,
        ),
        EventError::WrongDocument | EventError::Dom(_) => DispatchError::failed(
            CanonicalErrorCode::ActionPostconditionFailed,
            error.to_string(),
            false,
        ),
    }
}

/// The `interaction.click.v1` success result envelope: `{"schema":
/// "interaction.click.result.v1", "data": {...}}`. This is this task's
/// concrete instance of the minimal result-envelope convention the M1/M2
/// contract compatibility checklist recommended (finding 6) — no such
/// convention existed anywhere in the codebase before this change; this is
/// the first `CommandOutcome.result` payload with any internal structure.
/// Reports only caller-meaningful outcome facts (which default actions
/// fired, whether focus moved) — never a raw `NodeHandle` or other
/// internal-only representation.
fn click_result_envelope(outcome: &machina_events::ClickOutcome) -> String {
    serde_json::json!({
        "schema": "interaction.click.result.v1",
        "data": {
            "mousedownDefaultPrevented": outcome.mousedown_default_prevented,
            "mouseupDefaultPrevented": outcome.mouseup_default_prevented,
            "clickDefaultPrevented": outcome.click_default_prevented,
            "focusChanged": outcome.focused.is_some(),
        }
    })
    .to_string()
}

fn map_session_error(error: SessionError) -> DispatchError {
    match &error {
        SessionError::NotReady => DispatchError::failed(
            CanonicalErrorCode::SessionNotReady,
            error.to_string(),
            false,
        ),
        SessionError::Closed => {
            DispatchError::failed(CanonicalErrorCode::SessionClosed, error.to_string(), false)
        }
        SessionError::ResourceLimitExceeded { .. }
        | SessionError::PageResourceLimitExceeded { .. } => {
            DispatchError::failed(CanonicalErrorCode::QuotaExceeded, error.to_string(), false)
        }
        SessionError::InvalidIdentity(_)
        | SessionError::InvalidTransition { .. }
        | SessionError::ContextAlreadyExists
        | SessionError::ContextNotFound
        | SessionError::PageAlreadyExists
        | SessionError::PageNotFound
        | SessionError::ResourceNotReleasable(_) => DispatchError::failed(
            CanonicalErrorCode::InvalidArgument,
            error.to_string(),
            false,
        ),
    }
}

fn parse_session_id(value: &str) -> Result<SessionId, DispatchError> {
    SessionId::new(value.to_owned()).map_err(map_session_error)
}

fn parse_context_id(value: &str) -> Result<ContextId, DispatchError> {
    ContextId::new(value.to_owned()).map_err(map_session_error)
}

fn parse_page_id(value: &str) -> Result<PageId, DispatchError> {
    PageId::new(value.to_owned()).map_err(map_session_error)
}

struct LifecycleEngine {
    kind: EngineKind,
    snapshot: CapabilitySnapshot,
    sessions: Mutex<BTreeMap<SessionId, EngineSession>>,
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
        // `interaction.click.v1` is Native-only: this engine composes its
        // own in-memory `machina_dom::Document`, not a real Chromium
        // render, so advertising it as Chromium-supported here would let
        // the bus route real click commands to a fabricated result. Not
        // registering it for Chromium at all (rather than registering it
        // as `Unsupported`) means `CommandBus::decide` correctly reports
        // `no_compatible_engine`/`UNSUPPORTED_CAPABILITY` when no engine
        // can serve it, matching every other not-yet-implemented
        // capability's honest behavior. See this crate's module docs.
        if kind == EngineKind::Native {
            snapshot.register("interaction.click.v1", status);
        }
        Self {
            kind,
            snapshot,
            sessions: Mutex::new(BTreeMap::new()),
        }
    }

    fn lock_sessions(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<SessionId, EngineSession>>, DispatchError> {
        self.sessions.lock().map_err(|_| {
            DispatchError::failed(
                CanonicalErrorCode::CapacityUnavailable,
                "session registry lock is poisoned",
                true,
            )
        })
    }

    fn with_session_mut<T>(
        &self,
        session_id: &str,
        operation: impl FnOnce(&mut EngineSession) -> Result<T, SessionError>,
    ) -> Result<T, DispatchError> {
        let id = parse_session_id(session_id)?;
        let mut sessions = self.lock_sessions()?;
        let session = sessions.get_mut(&id).ok_or_else(|| {
            DispatchError::failed(
                CanonicalErrorCode::SessionClosed,
                "session does not exist or is already closed",
                false,
            )
        })?;
        operation(session).map_err(map_session_error)
    }

    /// Mutable access to a session's live document, for whichever layer
    /// populates content into it. Today: only this crate's own tests,
    /// exercising `interaction.click.v1` end to end against real elements.
    /// Eventually: M2-T09's `navigation.goto.v1` wiring, once merged — this
    /// method is the plumbing that task needs and does not yet have to add
    /// itself.
    fn with_document_mut<T>(
        &self,
        session_id: &str,
        operation: impl FnOnce(&mut Document) -> T,
    ) -> Result<T, DispatchError> {
        let id = parse_session_id(session_id)?;
        let mut sessions = self.lock_sessions()?;
        let session = sessions.get_mut(&id).ok_or_else(|| {
            DispatchError::failed(
                CanonicalErrorCode::SessionClosed,
                "session does not exist or is already closed",
                false,
            )
        })?;
        Ok(operation(session.document_mut()))
    }

    /// `interaction.click.v1`. Native-only (see `LifecycleEngine::new`'s
    /// registration comment and this crate's module docs): rejected
    /// explicitly, not silently faked, if reached on the Chromium engine.
    fn click(&self, session_id: &str, selector: &str) -> Result<String, DispatchError> {
        if self.kind != EngineKind::Native {
            return Err(DispatchError::unsupported("interaction.click.v1"));
        }
        let id = parse_session_id(session_id)?;
        let mut sessions = self.lock_sessions()?;
        let session = sessions.get_mut(&id).ok_or_else(|| {
            DispatchError::failed(
                CanonicalErrorCode::SessionClosed,
                "session does not exist or is already closed",
                false,
            )
        })?;
        if session.state() != LifecycleState::Ready {
            return Err(DispatchError::failed(
                CanonicalErrorCode::SessionNotReady,
                "session is not ready to accept interaction commands",
                false,
            ));
        }
        session.click(selector)
    }

    /// Open a browsing context under an existing, ready session.
    fn open_context(&self, session_id: &str, context_id: &str) -> Result<(), DispatchError> {
        let context_id = parse_context_id(context_id)?;
        self.with_session_mut(session_id, |session| session.open_context(context_id))
    }

    /// Close a browsing context and every page it owns.
    fn close_context(&self, session_id: &str, context_id: &str) -> Result<(), DispatchError> {
        let context_id = parse_context_id(context_id)?;
        self.with_session_mut(session_id, |session| session.close_context(&context_id))
    }

    /// Open a page under an existing, ready context, enforcing the
    /// session's aggregate page budget and the page's own resource budget.
    fn open_page(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        budget: PageResourceBudget,
    ) -> Result<(), DispatchError> {
        let context_id = parse_context_id(context_id)?;
        let page_id = parse_page_id(page_id)?;
        self.with_session_mut(session_id, |session| {
            session.open_page(&context_id, page_id, budget)
        })
    }

    /// Close a single page without closing its owning context.
    fn close_page(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
    ) -> Result<(), DispatchError> {
        let context_id = parse_context_id(context_id)?;
        let page_id = parse_page_id(page_id)?;
        self.with_session_mut(session_id, |session| {
            session.close_page(&context_id, &page_id)
        })
    }

    /// Account `amount` units of `kind` against a page's hard resource
    /// limit. Rejected atomically (usage untouched) once the budget for
    /// that category would be exceeded.
    fn account_page_resource(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), DispatchError> {
        let context_id = parse_context_id(context_id)?;
        let page_id = parse_page_id(page_id)?;
        self.with_session_mut(session_id, |session| {
            session.account_page_resource(&context_id, &page_id, kind, amount)
        })
    }

    /// Release `amount` units of a releasable page resource category.
    fn release_page_resource(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), DispatchError> {
        let context_id = parse_context_id(context_id)?;
        let page_id = parse_page_id(page_id)?;
        self.with_session_mut(session_id, |session| {
            session.release_page_resource(&context_id, &page_id, kind, amount)
        })
    }

    /// Cancel a session and every context/page it owns immediately,
    /// regardless of in-flight work.
    fn cancel_session(&self, session_id: &str) -> Result<(), DispatchError> {
        self.with_session_mut(session_id, |session| session.cancel())
    }

    fn health(&self) -> Result<EngineHealth, DispatchError> {
        let sessions = self.lock_sessions()?;
        Ok(EngineHealth {
            engine: self.kind,
            engine_build: self.snapshot.engine_build.clone(),
            capability_snapshot: self.snapshot.response(),
            sessions: sessions.values().map(EngineSession::health).collect(),
        })
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
                        CanonicalErrorCode::InvalidArgument,
                        "session.create.v1 requires a session-create payload",
                        false,
                    ));
                }
                let id = parse_session_id(&command.session_id)?;
                let mut sessions = self.lock_sessions()?;
                if sessions.contains_key(&id) {
                    return Err(DispatchError::failed(
                        CanonicalErrorCode::InvalidArgument,
                        "session already exists",
                        false,
                    ));
                }
                let mut session =
                    EngineSession::new(self.kind, id.clone(), ResourceBudget::default());
                session
                    .transition(LifecycleState::Ready)
                    .map_err(map_session_error)?;
                sessions.insert(id, session);
                Ok("session ready".to_owned())
            }
            CommandKind::SessionCloseV1 => {
                if !matches!(command.payload, CommandPayload::SessionClose(_)) {
                    return Err(DispatchError::failed(
                        CanonicalErrorCode::InvalidArgument,
                        "session.close.v1 requires a session-close payload",
                        false,
                    ));
                }
                let id = parse_session_id(&command.session_id)?;
                let mut sessions = self.lock_sessions()?;
                let mut session = sessions.remove(&id).ok_or_else(|| {
                    DispatchError::failed(
                        CanonicalErrorCode::SessionClosed,
                        "session does not exist or is already closed",
                        false,
                    )
                })?;
                // Closing cascades cancellation to every context/page the
                // session still owns; nothing is left orphaned.
                session
                    .transition(LifecycleState::Closing)
                    .map_err(map_session_error)?;
                session
                    .transition(LifecycleState::Closed)
                    .map_err(map_session_error)?;
                Ok("session closed".to_owned())
            }
            CommandKind::InteractionClickV1 => {
                let payload = match &command.payload {
                    CommandPayload::Click(payload) => payload,
                    _ => {
                        return Err(DispatchError::failed(
                            CanonicalErrorCode::InvalidArgument,
                            "interaction.click.v1 requires a click payload",
                            false,
                        ));
                    }
                };
                self.click(&command.session_id, &payload.selector)
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

    pub fn open_context(&self, session_id: &str, context_id: &str) -> Result<(), DispatchError> {
        self.inner.open_context(session_id, context_id)
    }

    pub fn close_context(&self, session_id: &str, context_id: &str) -> Result<(), DispatchError> {
        self.inner.close_context(session_id, context_id)
    }

    pub fn open_page(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        budget: PageResourceBudget,
    ) -> Result<(), DispatchError> {
        self.inner
            .open_page(session_id, context_id, page_id, budget)
    }

    pub fn close_page(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
    ) -> Result<(), DispatchError> {
        self.inner.close_page(session_id, context_id, page_id)
    }

    pub fn account_page_resource(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), DispatchError> {
        self.inner
            .account_page_resource(session_id, context_id, page_id, kind, amount)
    }

    pub fn release_page_resource(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), DispatchError> {
        self.inner
            .release_page_resource(session_id, context_id, page_id, kind, amount)
    }

    pub fn cancel_session(&self, session_id: &str) -> Result<(), DispatchError> {
        self.inner.cancel_session(session_id)
    }

    /// Mutable access to a session's live document, for populating content
    /// before an `interaction.click.v1` (or, eventually, a
    /// `navigation.goto.v1`) command is dispatched against it.
    pub fn with_document_mut<T>(
        &self,
        session_id: &str,
        operation: impl FnOnce(&mut Document) -> T,
    ) -> Result<T, DispatchError> {
        self.inner.with_document_mut(session_id, operation)
    }

    pub fn health(&self) -> Result<EngineHealth, DispatchError> {
        self.inner.health()
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

    // Session/context/page composition and resource accounting are shared
    // bookkeeping, not a live browser process, so the Chromium path is kept
    // symmetric with `NativeEngine` here. Chromium's actual browser launch
    // remains an explicit unsupported/injected boundary (see the shared
    // `execute` unsupported-capability fallthrough); this crate never fakes
    // that a page was actually rendered. `interaction.click.v1` follows the
    // same rule: `with_document_mut` is exposed symmetrically below (so a
    // future contributor is not tempted to special-case it), but a real
    // click against it is explicitly rejected in `LifecycleEngine::click`
    // (the capability is also never registered for this engine), not
    // silently run against `machina-dom`/`machina-events` as if it were a
    // real Chromium render.

    pub fn open_context(&self, session_id: &str, context_id: &str) -> Result<(), DispatchError> {
        self.inner.open_context(session_id, context_id)
    }

    pub fn close_context(&self, session_id: &str, context_id: &str) -> Result<(), DispatchError> {
        self.inner.close_context(session_id, context_id)
    }

    pub fn open_page(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        budget: PageResourceBudget,
    ) -> Result<(), DispatchError> {
        self.inner
            .open_page(session_id, context_id, page_id, budget)
    }

    pub fn close_page(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
    ) -> Result<(), DispatchError> {
        self.inner.close_page(session_id, context_id, page_id)
    }

    pub fn account_page_resource(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), DispatchError> {
        self.inner
            .account_page_resource(session_id, context_id, page_id, kind, amount)
    }

    pub fn release_page_resource(
        &self,
        session_id: &str,
        context_id: &str,
        page_id: &str,
        kind: PageResourceKind,
        amount: u64,
    ) -> Result<(), DispatchError> {
        self.inner
            .release_page_resource(session_id, context_id, page_id, kind, amount)
    }

    pub fn cancel_session(&self, session_id: &str) -> Result<(), DispatchError> {
        self.inner.cancel_session(session_id)
    }

    pub fn with_document_mut<T>(
        &self,
        session_id: &str,
        operation: impl FnOnce(&mut Document) -> T,
    ) -> Result<T, DispatchError> {
        self.inner.with_document_mut(session_id, operation)
    }

    pub fn health(&self) -> Result<EngineHealth, DispatchError> {
        self.inner.health()
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
    use machina_command_bus::{CommandBus, CommandContext, EngineAdapter, FallbackPolicy};
    use machina_command_model::{
        CanonicalErrorCode, ClickPayload, CommandEnvelope, CommandKind, CommandMetadata,
        CommandPayload, SessionClosePayload, SessionCreatePayload,
    };
    use machina_session::{LifecycleState, PageResourceBudget, PageResourceKind};
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

    fn click_command(session_id: &str, selector: &str) -> CommandEnvelope {
        CommandEnvelope {
            command_id: format!("{session_id}-click-command"),
            session_id: session_id.to_owned(),
            context_id: None,
            page_id: None,
            kind: CommandKind::InteractionClickV1,
            payload: CommandPayload::Click(ClickPayload {
                selector: selector.to_owned(),
            }),
            idempotency_key: None,
            deadline_ms: 5_000,
            required_capabilities: vec!["interaction.click.v1".to_owned()],
            metadata: CommandMetadata {
                correlation_id: format!("{session_id}-click-correlation"),
                causation_id: None,
                client: "native-core-test".to_owned(),
            },
        }
    }

    fn tiny_page_budget() -> PageResourceBudget {
        PageResourceBudget {
            max_dom_nodes: 1,
            max_timers: 1,
            max_event_listeners: 1,
            max_network_requests: 1,
            max_network_bytes: 1,
            max_artifacts: 1,
        }
    }

    /// Creates a session directly against one engine adapter instance
    /// (bypassing the bus, which would own a *different* pair of engines)
    /// so context/page composition can be exercised against the same
    /// instance the test asserts on afterward.
    fn create_session_directly(engine: &impl EngineAdapter, session_id: &str) {
        let context = CommandContext::with_timeout(
            format!("{session_id}-correlation"),
            Duration::from_secs(1),
        );
        engine
            .execute(&command(CommandKind::SessionCreateV1, session_id), &context)
            .expect("session creation should succeed");
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
        assert_eq!(error.code, CanonicalErrorCode::UnsupportedCapability);
    }

    /// Acceptance: create/close/cancel transitions match the canonical
    /// contract at every identity level (session, context, page), exercised
    /// through the native engine composition end to end.
    #[test]
    fn engine_session_create_close_cancel_transitions_match_canonical_contract() {
        let engine = NativeEngine::new("native-test");

        // A context/page cannot be created before the session exists.
        let error = engine
            .open_context("session-1", "context-1")
            .expect_err("context cannot be created before the session exists");
        assert_eq!(error.code, CanonicalErrorCode::SessionClosed);

        // Create.
        create_session_directly(&engine, "session-1");
        engine
            .open_context("session-1", "context-1")
            .expect("context creation succeeds once the session is ready");
        engine
            .open_page(
                "session-1",
                "context-1",
                "page-1",
                PageResourceBudget::default(),
            )
            .expect("page creation succeeds under a ready context");

        // Close: graceful teardown via context/page-scoped close calls.
        engine
            .close_page("session-1", "context-1", "page-1")
            .expect("page close succeeds");
        engine
            .close_context("session-1", "context-1")
            .expect("context close succeeds");

        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        engine
            .execute(&command(CommandKind::SessionCloseV1, "session-1"), &context)
            .expect("session close succeeds");

        // Cancel is a distinct path from close: it forces Ready -> Failed
        // immediately instead of the graceful Ready -> Closing -> Closed
        // sequence, and it is only reachable while the session still
        // exists.
        create_session_directly(&engine, "session-2");
        engine
            .cancel_session("session-2")
            .expect("cancel is a valid transition from ready");
        let health = engine.health().expect("health should be available");
        let session_health = health
            .sessions
            .iter()
            .find(|session| session.session_id == "session-2")
            .expect("cancelled session is still tracked until closed");
        assert_eq!(session_health.state, LifecycleState::Failed);

        // Cancelling an already-nonexistent session is reported explicitly,
        // never silently treated as success.
        let error = engine
            .cancel_session("session-does-not-exist")
            .expect_err("cancelling a session that does not exist must fail explicitly");
        assert_eq!(error.code, CanonicalErrorCode::SessionClosed);
    }

    /// Acceptance: cancelling a session cascades cancellation to every
    /// context/page it owns, and further resource accounting against a
    /// cancelled page is rejected rather than silently accepted.
    #[test]
    fn cancelling_a_session_cascades_and_blocks_further_resource_accounting() {
        let engine = NativeEngine::new("native-test");
        create_session_directly(&engine, "session-1");
        engine
            .open_context("session-1", "context-1")
            .expect("context creation succeeds");
        engine
            .open_page(
                "session-1",
                "context-1",
                "page-1",
                PageResourceBudget::default(),
            )
            .expect("page creation succeeds");
        engine
            .account_page_resource(
                "session-1",
                "context-1",
                "page-1",
                PageResourceKind::DomNodes,
                5,
            )
            .expect("resource accounting succeeds before cancellation");

        engine
            .cancel_session("session-1")
            .expect("cancel is a valid transition from ready");

        let error = engine
            .account_page_resource(
                "session-1",
                "context-1",
                "page-1",
                PageResourceKind::DomNodes,
                1,
            )
            .expect_err("a cancelled page must reject further resource accounting");
        assert_eq!(error.code, CanonicalErrorCode::SessionNotReady);
    }

    /// Acceptance: every page resource category enforces a hard limit, and
    /// exhausting it produces an explicit quota error rather than silent
    /// truncation, wraparound, or success.
    #[test]
    fn forced_budget_exhaustion_is_explicit_for_every_page_resource_category() {
        let engine = NativeEngine::new("native-test");
        create_session_directly(&engine, "session-1");
        engine
            .open_context("session-1", "context-1")
            .expect("context creation succeeds");
        engine
            .open_page("session-1", "context-1", "page-1", tiny_page_budget())
            .expect("page creation succeeds");

        for kind in [
            PageResourceKind::DomNodes,
            PageResourceKind::Timers,
            PageResourceKind::EventListeners,
            PageResourceKind::NetworkRequests,
            PageResourceKind::NetworkBytes,
            PageResourceKind::Artifacts,
        ] {
            engine
                .account_page_resource("session-1", "context-1", "page-1", kind, 1)
                .unwrap_or_else(|error| {
                    panic!("first unit of {kind:?} must fit the budget: {error}")
                });
            let error = engine
                .account_page_resource("session-1", "context-1", "page-1", kind, 1)
                .expect_err("second unit must be rejected once the hard limit is exhausted");
            assert_eq!(
                error.code,
                CanonicalErrorCode::QuotaExceeded,
                "category {kind:?}"
            );
        }
    }

    /// Acceptance: exceeding the *session-level* aggregate page/context
    /// budget is also a hard limit, independent of any single page's own
    /// resource categories.
    #[test]
    fn forced_budget_exhaustion_is_explicit_for_session_level_context_and_page_counters() {
        let engine = NativeEngine::new("native-test");
        create_session_directly(&engine, "session-1");

        for index in 0..4 {
            engine
                .open_context("session-1", &format!("context-{index}"))
                .unwrap_or_else(|error| {
                    panic!("context {index} should fit the default budget: {error}")
                });
        }
        let error = engine
            .open_context("session-1", "context-overflow")
            .expect_err("session-level context budget is a hard limit");
        assert_eq!(error.code, CanonicalErrorCode::QuotaExceeded);

        for index in 0..8 {
            engine
                .open_page(
                    "session-1",
                    "context-0",
                    &format!("page-{index}"),
                    PageResourceBudget::default(),
                )
                .unwrap_or_else(|error| {
                    panic!("page {index} should fit the default budget: {error}")
                });
        }
        let error = engine
            .open_page(
                "session-1",
                "context-0",
                "page-overflow",
                PageResourceBudget::default(),
            )
            .expect_err("session-level aggregate page budget is a hard limit");
        assert_eq!(error.code, CanonicalErrorCode::QuotaExceeded);
    }

    /// Acceptance: capability snapshot and per-session health are exposed
    /// to the worker/scheduler layer through a single call, with no
    /// protocol-layer type in the returned shape.
    #[test]
    fn health_exposes_capability_snapshot_and_session_state() {
        let engine = NativeEngine::new("native-test");
        create_session_directly(&engine, "session-1");
        engine
            .open_context("session-1", "context-1")
            .expect("context creation succeeds");
        engine
            .open_page(
                "session-1",
                "context-1",
                "page-1",
                PageResourceBudget::default(),
            )
            .expect("page creation succeeds");

        let health = engine.health().expect("health should be available");
        assert_eq!(health.engine, machina_command_model::EngineKind::Native);
        assert!(health
            .capability_snapshot
            .capabilities
            .iter()
            .any(|entry| entry.id == "session.create.v1"));
        let session_health = health
            .sessions
            .iter()
            .find(|session| session.session_id == "session-1")
            .expect("created session is reported in health");
        assert_eq!(session_health.state, LifecycleState::Ready);
        assert_eq!(session_health.context_count, 1);
        assert_eq!(session_health.page_count, 1);
    }

    /// Acceptance: native and Chromium engines compose context/page
    /// identities through the same contract shape (create/close, aggregate
    /// budgets, per-page resource accounting). Chromium's actual browser
    /// launch remains unimplemented and is never faked by this test; only
    /// the shared composition/accounting boundary is asserted.
    #[test]
    fn native_and_chromium_engines_compose_context_and_page_identities_symmetrically() {
        let native = NativeEngine::new("native-test");
        create_session_directly(&native, "session-1");
        native
            .open_context("session-1", "context-1")
            .expect("native context creation succeeds");
        native
            .open_page(
                "session-1",
                "context-1",
                "page-1",
                PageResourceBudget::default(),
            )
            .expect("native page creation succeeds");
        native
            .account_page_resource(
                "session-1",
                "context-1",
                "page-1",
                PageResourceKind::DomNodes,
                10,
            )
            .expect("native resource accounting succeeds within budget");
        native
            .close_page("session-1", "context-1", "page-1")
            .expect("native page close succeeds");
        native
            .close_context("session-1", "context-1")
            .expect("native context close succeeds");

        let chromium = ChromiumEngine::new("chromium-test");
        create_session_directly(&chromium, "session-1");
        chromium
            .open_context("session-1", "context-1")
            .expect("chromium context creation succeeds");
        chromium
            .open_page(
                "session-1",
                "context-1",
                "page-1",
                PageResourceBudget::default(),
            )
            .expect("chromium page creation succeeds");
        chromium
            .account_page_resource(
                "session-1",
                "context-1",
                "page-1",
                PageResourceKind::DomNodes,
                10,
            )
            .expect("chromium resource accounting succeeds within budget");
        chromium
            .close_page("session-1", "context-1", "page-1")
            .expect("chromium page close succeeds");
        chromium
            .close_context("session-1", "context-1")
            .expect("chromium context close succeeds");
    }

    /// Acceptance: `interaction.click.v1` routed through the shared
    /// `CommandBus` resolves a real, attached element and returns the
    /// `interaction.click.result.v1` envelope, not an unsupported-capability
    /// error — the concrete behavior change this task implements.
    #[test]
    fn interaction_click_v1_routes_through_the_bus_and_resolves_a_real_element() {
        let native = NativeEngine::new("native-test");
        create_session_directly(&native, "session-1");
        native
            .with_document_mut("session-1", |document| {
                let button = document.create_element("button").expect("create button");
                document
                    .set_attribute(button, "id", "submit")
                    .expect("set id attribute");
                let root = document.root();
                document
                    .append_child(root, button.node_handle())
                    .expect("attach button to document");
            })
            .expect("document population succeeds");

        let bus = CommandBus::new(
            native,
            ChromiumEngine::new("chromium-test"),
            FallbackPolicy::PreferNative,
        );
        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        let outcome = bus
            .execute(&click_command("session-1", "#submit"), &context)
            .expect("click on a real, attached element should succeed");
        assert_eq!(
            outcome.execution.engine,
            machina_command_model::EngineKind::Native
        );
        let result = outcome
            .result
            .expect("a successful click reports a result envelope");
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("result envelope is valid json");
        assert_eq!(parsed["schema"], "interaction.click.result.v1");
        assert_eq!(parsed["data"]["mousedownDefaultPrevented"], false);
        assert_eq!(parsed["data"]["mouseupDefaultPrevented"], false);
        assert_eq!(parsed["data"]["clickDefaultPrevented"], false);
    }

    /// Acceptance: a selector matching nothing is `ELEMENT_NOT_FOUND`, not
    /// `UNSUPPORTED_CAPABILITY` — the deliberate, reviewed behavior change
    /// from the M1 baseline (see `scripts/test/m1-compatibility-smoke.mjs`).
    #[test]
    fn interaction_click_v1_missing_selector_is_element_not_found() {
        let engine = NativeEngine::new("native-test");
        create_session_directly(&engine, "session-1");
        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        let error = engine
            .execute(&click_command("session-1", "#missing"), &context)
            .expect_err("a selector matching nothing must fail explicitly");
        assert_eq!(error.code, CanonicalErrorCode::ElementNotFound);
    }

    /// Acceptance: a selector matching more than one element is
    /// `ELEMENT_AMBIGUOUS`. Design decision: `interaction.click.v1` uses
    /// `query_selector_all` (not `query_selector`'s silent first-match) and
    /// checks the match count itself, because a click is a real,
    /// side-effecting action — silently clicking whichever element happened
    /// to come first in document order would be the wrong default for an
    /// automation product (see this crate's module docs).
    #[test]
    fn interaction_click_v1_ambiguous_selector_is_element_ambiguous() {
        let engine = NativeEngine::new("native-test");
        create_session_directly(&engine, "session-1");
        engine
            .with_document_mut("session-1", |document| {
                // A document node allows at most one `Element` child
                // (matches real DOM's single `documentElement` rule), so the
                // two ambiguous siblings are attached under one wrapping
                // `ul` element rather than directly under the document root.
                let root = document.root();
                let list = document.create_element("ul").expect("create ul");
                document
                    .append_child(root, list.node_handle())
                    .expect("attach ul to document");
                for _ in 0..2 {
                    let element = document.create_element("li").expect("create li");
                    document
                        .set_attribute(element, "class", "item")
                        .expect("set class attribute");
                    document
                        .append_child(list.node_handle(), element.node_handle())
                        .expect("attach li to list");
                }
            })
            .expect("document population succeeds");

        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        let error = engine
            .execute(&click_command("session-1", ".item"), &context)
            .expect_err("a selector matching more than one element must fail explicitly");
        assert_eq!(error.code, CanonicalErrorCode::ElementAmbiguous);
    }

    /// Acceptance: malformed selector syntax is `SELECTOR_INVALID` (the
    /// pinned `CanonicalErrorCode` variant that exists for exactly this
    /// case), never treated as "matched nothing."
    #[test]
    fn interaction_click_v1_malformed_selector_is_selector_invalid() {
        let engine = NativeEngine::new("native-test");
        create_session_directly(&engine, "session-1");
        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        let error = engine
            .execute(&click_command("session-1", ""), &context)
            .expect_err("an empty selector string is malformed, not a legitimate empty match");
        assert_eq!(error.code, CanonicalErrorCode::SelectorInvalid);
    }

    /// Acceptance: an unready session rejects `interaction.click.v1`
    /// explicitly (`SESSION_NOT_READY`), matching every other command kind's
    /// session-state precondition rather than silently no-opting.
    #[test]
    fn interaction_click_v1_requires_a_ready_session() {
        let engine = NativeEngine::new("native-test");
        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        let error = engine
            .execute(&click_command("session-1", "#missing"), &context)
            .expect_err("a session that was never created cannot accept a click");
        assert_eq!(error.code, CanonicalErrorCode::SessionClosed);
    }

    /// Acceptance / documented design decision: `interaction.click.v1` is
    /// Native-only. The Chromium engine never registers the capability (so
    /// `CommandBus::decide` reports `UNSUPPORTED_CAPABILITY` rather than
    /// routing to a fabricated result), and a direct call that bypasses the
    /// bus is also rejected explicitly by `LifecycleEngine::click`'s own
    /// engine-kind guard — this crate never fakes a Chromium click against
    /// its native-only `machina-dom`/`machina-events` composition.
    #[test]
    fn interaction_click_v1_is_native_only_and_never_faked_on_chromium() {
        let chromium = ChromiumEngine::new("chromium-test");
        assert!(
            !chromium.capabilities().supports("interaction.click.v1"),
            "the chromium engine must not advertise interaction.click.v1 as supported"
        );
        create_session_directly(&chromium, "session-1");
        let context = CommandContext::with_timeout("session-1-correlation", Duration::from_secs(1));
        let error = chromium
            .execute(&click_command("session-1", "#missing"), &context)
            .expect_err("a direct call bypassing the bus must still be rejected explicitly");
        assert_eq!(error.code, CanonicalErrorCode::UnsupportedCapability);

        let bus = CommandBus::new(
            NativeEngine::new("native-test"),
            ChromiumEngine::new("chromium-test"),
            FallbackPolicy::ChromiumOnly,
        );
        let routed = bus.execute(&click_command("session-1", "#missing"), &context);
        let error = routed.expect_err("no engine can honestly serve this under chromium-only");
        assert_eq!(error.code, CanonicalErrorCode::UnsupportedCapability);
    }

    /// Acceptance: `perform_click`'s `EventError` variants map onto
    /// documented `CanonicalErrorCode`s. `NotInteractable` cannot actually be
    /// reached end-to-end through `interaction.click.v1` today (selector
    /// resolution only ever returns elements already reachable from the
    /// document root, which is exactly `perform_click`'s own attachment
    /// precondition — see `.agent-state/evidence/wire-interaction-click.md`),
    /// so this unit-tests the mapping function directly rather than
    /// asserting an unreachable end-to-end path.
    #[test]
    fn map_event_error_covers_not_interactable() {
        let error = super::map_event_error(machina_events::EventError::NotInteractable);
        assert_eq!(error.code, CanonicalErrorCode::ElementNotInteractable);
    }
}
