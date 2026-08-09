//! Lifecycle, identity, cancellation, and bounded-resource primitives shared
//! by session, browsing-context, and page composition in the native engine.
//!
//! This crate is a foundation-layer module: it knows about identities, state
//! transitions, cancellation, and resource counters only. It never imports
//! protocol/control-plane types and never encodes browser semantics (DOM,
//! navigation, network). Limits fail explicitly before an operation can
//! exceed its budget; nothing here silently clamps or drops usage.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use machina_command_bus::CancellationToken;

/// Canonical identity for a browser automation session.
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

/// Canonical identity for a browsing context owned by a session (a browser
/// context in the CDP/BiDi sense: an isolated storage/cookie partition that
/// owns zero or more pages).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ContextId(String);

impl ContextId {
    pub fn new(value: impl Into<String>) -> Result<Self, SessionError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SessionError::InvalidIdentity(
                "context id must not be empty",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Canonical identity for a page owned by exactly one browsing context.
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

/// Lifecycle state shared by sessions, contexts, and pages. The transition
/// contract is identical at every identity level: a create path
/// (`Creating -> Ready`), a close path (`Ready -> Closing -> Closed`), and an
/// explicit cancel path (`Creating|Ready -> Failed -> Closed`) that a caller
/// can invoke immediately instead of waiting for a graceful close.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    Creating,
    Ready,
    Closing,
    Closed,
    Failed,
}

/// Backward/forward-compatible alias: sessions, contexts, and pages all use
/// the same canonical lifecycle contract.
pub type SessionState = LifecycleState;
pub type ContextState = LifecycleState;
pub type PageState = LifecycleState;

impl LifecycleState {
    fn validate_transition(from: Self, to: Self) -> bool {
        matches!(
            (from, to),
            (Self::Creating, Self::Ready)
                | (Self::Creating, Self::Failed)
                | (Self::Ready, Self::Closing)
                | (Self::Ready, Self::Failed)
                | (Self::Closing, Self::Closed)
                | (Self::Failed, Self::Closed)
        )
    }

    /// True once the identity can no longer accept new work of any kind.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closed)
    }

    /// True once the identity is winding down (closing, cancelled, or
    /// closed); cancellation must be visible immediately in this state.
    pub fn is_cancelling_or_terminal(self) -> bool {
        matches!(self, Self::Closing | Self::Closed | Self::Failed)
    }
}

/// Session-level bounded counters: how many contexts and how many pages
/// (aggregated across all contexts) a session may hold concurrently.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceBudget {
    pub max_contexts: u32,
    pub max_pages: u32,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_contexts: 4,
            max_pages: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceUsage {
    pub contexts: u32,
    pub pages: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    Contexts,
    Pages,
}

/// Every accounted resource category a single page can exhaust. Categories
/// are intentionally engine-facing (not protocol-facing): they describe what
/// the native engine must bound while parsing, scripting, and loading a
/// page, independent of which wire protocol requested the work.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PageResourceKind {
    DomNodes,
    Timers,
    EventListeners,
    NetworkRequests,
    NetworkBytes,
    Artifacts,
}

impl PageResourceKind {
    /// Categories that represent a point-in-time count (nodes still
    /// attached, timers still armed, listeners still registered) and can
    /// legitimately be released. Network and artifact usage are cumulative
    /// for the life of the page and are never released.
    pub fn is_releasable(self) -> bool {
        matches!(self, Self::DomNodes | Self::Timers | Self::EventListeners)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageResourceBudget {
    pub max_dom_nodes: u64,
    pub max_timers: u32,
    pub max_event_listeners: u32,
    pub max_network_requests: u64,
    pub max_network_bytes: u64,
    pub max_artifacts: u32,
}

impl Default for PageResourceBudget {
    fn default() -> Self {
        Self {
            max_dom_nodes: 200_000,
            max_timers: 512,
            max_event_listeners: 4_096,
            max_network_requests: 1_000,
            max_network_bytes: 64 * 1024 * 1024,
            max_artifacts: 100,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PageResourceUsage {
    pub dom_nodes: u64,
    pub timers: u32,
    pub event_listeners: u32,
    pub network_requests: u64,
    pub network_bytes: u64,
    pub artifacts: u32,
}

impl PageResourceUsage {
    fn get(&self, kind: PageResourceKind) -> u64 {
        match kind {
            PageResourceKind::DomNodes => self.dom_nodes,
            PageResourceKind::Timers => u64::from(self.timers),
            PageResourceKind::EventListeners => u64::from(self.event_listeners),
            PageResourceKind::NetworkRequests => self.network_requests,
            PageResourceKind::NetworkBytes => self.network_bytes,
            PageResourceKind::Artifacts => u64::from(self.artifacts),
        }
    }

    fn set(&mut self, kind: PageResourceKind, value: u64) {
        match kind {
            PageResourceKind::DomNodes => self.dom_nodes = value,
            PageResourceKind::Timers => self.timers = value as u32,
            PageResourceKind::EventListeners => self.event_listeners = value as u32,
            PageResourceKind::NetworkRequests => self.network_requests = value,
            PageResourceKind::NetworkBytes => self.network_bytes = value,
            PageResourceKind::Artifacts => self.artifacts = value as u32,
        }
    }
}

fn page_budget_limit(budget: PageResourceBudget, kind: PageResourceKind) -> u64 {
    match kind {
        PageResourceKind::DomNodes => budget.max_dom_nodes,
        PageResourceKind::Timers => u64::from(budget.max_timers),
        PageResourceKind::EventListeners => u64::from(budget.max_event_listeners),
        PageResourceKind::NetworkRequests => budget.max_network_requests,
        PageResourceKind::NetworkBytes => budget.max_network_bytes,
        PageResourceKind::Artifacts => u64::from(budget.max_artifacts),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    InvalidIdentity(&'static str),
    InvalidTransition {
        from: LifecycleState,
        to: LifecycleState,
    },
    NotReady,
    Closed,
    ContextAlreadyExists,
    ContextNotFound,
    PageAlreadyExists,
    PageNotFound,
    ResourceLimitExceeded {
        kind: ResourceKind,
        limit: u64,
    },
    PageResourceLimitExceeded {
        kind: PageResourceKind,
        limit: u64,
    },
    ResourceNotReleasable(PageResourceKind),
}

impl Display for SessionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity(message) => formatter.write_str(message),
            Self::InvalidTransition { from, to } => {
                write!(
                    formatter,
                    "invalid lifecycle transition: {from:?} -> {to:?}"
                )
            }
            Self::NotReady => formatter.write_str("identity is not ready"),
            Self::Closed => formatter.write_str("identity is closed"),
            Self::ContextAlreadyExists => formatter.write_str("context already exists"),
            Self::ContextNotFound => formatter.write_str("context not found"),
            Self::PageAlreadyExists => formatter.write_str("page already exists"),
            Self::PageNotFound => formatter.write_str("page not found"),
            Self::ResourceLimitExceeded { kind, limit } => {
                write!(formatter, "resource limit exceeded for {kind:?}: {limit}")
            }
            Self::PageResourceLimitExceeded { kind, limit } => {
                write!(
                    formatter,
                    "page resource limit exceeded for {kind:?}: {limit}"
                )
            }
            Self::ResourceNotReleasable(kind) => {
                write!(
                    formatter,
                    "page resource category is not releasable: {kind:?}"
                )
            }
        }
    }
}

impl std::error::Error for SessionError {}

/// A single page owned by exactly one browsing context. Owns its own
/// lifecycle state, cancellation token, and per-category resource counters.
#[derive(Debug)]
pub struct Page {
    id: PageId,
    context_id: ContextId,
    state: LifecycleState,
    budget: PageResourceBudget,
    usage: PageResourceUsage,
    cancellation: CancellationToken,
}

impl Page {
    fn new(id: PageId, context_id: ContextId, budget: PageResourceBudget) -> Self {
        Self {
            id,
            context_id,
            state: LifecycleState::Creating,
            budget,
            usage: PageResourceUsage::default(),
            cancellation: CancellationToken::new(),
        }
    }

    pub fn id(&self) -> &PageId {
        &self.id
    }

    pub fn context_id(&self) -> &ContextId {
        &self.context_id
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn budget(&self) -> PageResourceBudget {
        self.budget
    }

    pub fn usage(&self) -> PageResourceUsage {
        self.usage
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn transition(&mut self, next: LifecycleState) -> Result<(), SessionError> {
        if !LifecycleState::validate_transition(self.state, next) {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        if next.is_cancelling_or_terminal() {
            self.cancellation.cancel();
        }
        Ok(())
    }

    /// Cancel this page immediately regardless of in-flight work. Valid from
    /// `Creating` or `Ready`; a no-op error if already winding down.
    pub fn cancel(&mut self) -> Result<(), SessionError> {
        self.transition(LifecycleState::Failed)
    }

    /// Account `amount` units of `kind` against this page's hard limit. The
    /// operation is atomic: on rejection, usage is left unchanged.
    pub fn account(&mut self, kind: PageResourceKind, amount: u64) -> Result<(), SessionError> {
        if self.state != LifecycleState::Ready {
            return Err(SessionError::NotReady);
        }
        let limit = page_budget_limit(self.budget, kind);
        let current = self.usage.get(kind);
        let updated = current
            .checked_add(amount)
            .ok_or(SessionError::PageResourceLimitExceeded { kind, limit })?;
        if updated > limit {
            return Err(SessionError::PageResourceLimitExceeded { kind, limit });
        }
        self.usage.set(kind, updated);
        Ok(())
    }

    /// Release `amount` units of a releasable category (e.g. DOM nodes
    /// detached, timers cleared, listeners removed). Cumulative categories
    /// (network requests/bytes, artifacts) reject release explicitly instead
    /// of silently under-reporting usage.
    pub fn release(&mut self, kind: PageResourceKind, amount: u64) -> Result<(), SessionError> {
        if !kind.is_releasable() {
            return Err(SessionError::ResourceNotReleasable(kind));
        }
        let current = self.usage.get(kind);
        self.usage.set(kind, current.saturating_sub(amount));
        Ok(())
    }
}

/// A browsing context: an isolated partition that owns zero or more pages.
#[derive(Debug)]
pub struct Context {
    id: ContextId,
    state: LifecycleState,
    pages: BTreeMap<PageId, Page>,
    cancellation: CancellationToken,
}

impl Context {
    fn new(id: ContextId) -> Self {
        Self {
            id,
            state: LifecycleState::Creating,
            pages: BTreeMap::new(),
            cancellation: CancellationToken::new(),
        }
    }

    pub fn id(&self) -> &ContextId {
        &self.id
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn page(&self, id: &PageId) -> Option<&Page> {
        self.pages.get(id)
    }

    pub fn page_mut(&mut self, id: &PageId) -> Option<&mut Page> {
        self.pages.get_mut(id)
    }

    pub fn pages(&self) -> impl Iterator<Item = &Page> {
        self.pages.values()
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn transition(&mut self, next: LifecycleState) -> Result<(), SessionError> {
        if !LifecycleState::validate_transition(self.state, next) {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        if next.is_cancelling_or_terminal() {
            self.cascade_cancel();
        }
        Ok(())
    }

    /// Cancel this context and every page it owns immediately.
    pub fn cancel(&mut self) -> Result<(), SessionError> {
        self.transition(LifecycleState::Failed)
    }

    fn cascade_cancel(&mut self) {
        self.cancellation.cancel();
        for page in self.pages.values_mut() {
            if page.state().is_cancelling_or_terminal() {
                page.cancellation.cancel();
            } else {
                let _ = page.transition(LifecycleState::Failed);
            }
        }
    }

    fn open_page(&mut self, id: PageId, budget: PageResourceBudget) -> Result<(), SessionError> {
        if self.state != LifecycleState::Ready {
            return Err(SessionError::NotReady);
        }
        if self.pages.contains_key(&id) {
            return Err(SessionError::PageAlreadyExists);
        }
        let mut page = Page::new(id.clone(), self.id.clone(), budget);
        page.transition(LifecycleState::Ready)?;
        self.pages.insert(id, page);
        Ok(())
    }

    fn close_page(&mut self, id: &PageId) -> Result<(), SessionError> {
        let page = self.pages.get_mut(id).ok_or(SessionError::PageNotFound)?;
        if page.state() == LifecycleState::Ready {
            page.transition(LifecycleState::Closing)?;
        }
        if matches!(
            page.state(),
            LifecycleState::Closing | LifecycleState::Failed
        ) {
            page.transition(LifecycleState::Closed)?;
        }
        self.pages.remove(id);
        Ok(())
    }
}

/// The canonical native-engine session composition: a session identity, its
/// lifecycle, its cancellation scope, and the browsing contexts/pages it
/// owns together with their bounded resource counters. This is the facade
/// the engine adapter composes per active session; it never carries
/// protocol- or control-plane-specific types.
pub struct Session {
    id: SessionId,
    state: LifecycleState,
    budget: ResourceBudget,
    usage: ResourceUsage,
    contexts: BTreeMap<ContextId, Context>,
    cancellation: CancellationToken,
}

impl Session {
    pub fn new(id: SessionId, budget: ResourceBudget) -> Self {
        Self {
            id,
            state: LifecycleState::Creating,
            budget,
            usage: ResourceUsage::default(),
            contexts: BTreeMap::new(),
            cancellation: CancellationToken::new(),
        }
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn state(&self) -> LifecycleState {
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

    pub fn context(&self, id: &ContextId) -> Option<&Context> {
        self.contexts.get(id)
    }

    pub fn context_mut(&mut self, id: &ContextId) -> Option<&mut Context> {
        self.contexts.get_mut(id)
    }

    pub fn contexts(&self) -> impl Iterator<Item = &Context> {
        self.contexts.values()
    }

    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }

    pub fn transition(&mut self, next: LifecycleState) -> Result<(), SessionError> {
        if !LifecycleState::validate_transition(self.state, next) {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        if next.is_cancelling_or_terminal() {
            self.cascade_cancel();
        }
        Ok(())
    }

    /// Cancel the session and every context/page it owns immediately,
    /// regardless of in-flight work. Valid from `Creating` or `Ready`.
    pub fn cancel(&mut self) -> Result<(), SessionError> {
        self.transition(LifecycleState::Failed)
    }

    fn cascade_cancel(&mut self) {
        self.cancellation.cancel();
        for context in self.contexts.values_mut() {
            if !context.state().is_cancelling_or_terminal() {
                let _ = context.transition(LifecycleState::Failed);
            } else {
                context.cascade_cancel();
            }
        }
    }

    /// Open a new browsing context, enforcing the session's context budget.
    pub fn open_context(&mut self, id: ContextId) -> Result<&mut Context, SessionError> {
        if self.state != LifecycleState::Ready {
            return Err(SessionError::NotReady);
        }
        if self.contexts.contains_key(&id) {
            return Err(SessionError::ContextAlreadyExists);
        }
        if self.usage.contexts >= self.budget.max_contexts {
            return Err(SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Contexts,
                limit: u64::from(self.budget.max_contexts),
            });
        }
        let mut context = Context::new(id.clone());
        context.transition(LifecycleState::Ready)?;
        self.contexts.insert(id.clone(), context);
        self.usage.contexts += 1;
        Ok(self.contexts.get_mut(&id).expect("just inserted"))
    }

    /// Close a browsing context and every page it owns.
    pub fn close_context(&mut self, id: &ContextId) -> Result<(), SessionError> {
        let context = self
            .contexts
            .get_mut(id)
            .ok_or(SessionError::ContextNotFound)?;
        let page_ids: Vec<PageId> = context.pages.keys().cloned().collect();
        let closed_page_count = page_ids.len() as u32;
        for page_id in page_ids {
            context.close_page(&page_id)?;
        }
        self.usage.pages = self.usage.pages.saturating_sub(closed_page_count);
        if context.state() == LifecycleState::Ready {
            context.transition(LifecycleState::Closing)?;
        }
        if matches!(
            context.state(),
            LifecycleState::Closing | LifecycleState::Failed
        ) {
            context.transition(LifecycleState::Closed)?;
        }
        self.contexts.remove(id);
        self.usage.contexts = self.usage.contexts.saturating_sub(1);
        Ok(())
    }

    /// Open a page under an existing, ready context, enforcing the
    /// session-wide aggregate page budget.
    pub fn open_page(
        &mut self,
        context_id: &ContextId,
        page_id: PageId,
        page_budget: PageResourceBudget,
    ) -> Result<(), SessionError> {
        if self.state != LifecycleState::Ready {
            return Err(SessionError::NotReady);
        }
        if self.usage.pages >= self.budget.max_pages {
            return Err(SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Pages,
                limit: u64::from(self.budget.max_pages),
            });
        }
        let context = self
            .contexts
            .get_mut(context_id)
            .ok_or(SessionError::ContextNotFound)?;
        context.open_page(page_id, page_budget)?;
        self.usage.pages += 1;
        Ok(())
    }

    /// Close a single page without closing its owning context.
    pub fn close_page(
        &mut self,
        context_id: &ContextId,
        page_id: &PageId,
    ) -> Result<(), SessionError> {
        let context = self
            .contexts
            .get_mut(context_id)
            .ok_or(SessionError::ContextNotFound)?;
        context.close_page(page_id)?;
        self.usage.pages = self.usage.pages.saturating_sub(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Context, ContextId, LifecycleState, Page, PageId, PageResourceBudget, PageResourceKind,
        ResourceBudget, ResourceKind, Session, SessionError, SessionId,
    };

    fn ready_session(budget: ResourceBudget) -> Session {
        let mut session = Session::new(SessionId::new("session-1").expect("valid id"), budget);
        session
            .transition(LifecycleState::Ready)
            .expect("creating -> ready");
        session
    }

    fn tiny_page_budget() -> PageResourceBudget {
        PageResourceBudget {
            max_dom_nodes: 1,
            max_timers: 1,
            max_event_listeners: 1,
            max_network_requests: 1,
            max_network_bytes: 10,
            max_artifacts: 1,
        }
    }

    #[test]
    fn session_create_close_cancel_transitions_match_canonical_contract() {
        let mut session = Session::new(
            SessionId::new("session-1").expect("valid id"),
            ResourceBudget::default(),
        );
        assert_eq!(session.state(), LifecycleState::Creating);

        // Create path.
        session
            .transition(LifecycleState::Ready)
            .expect("creating -> ready");
        assert_eq!(session.state(), LifecycleState::Ready);

        // Invalid: cannot skip straight to closed.
        let mut skip_attempt = Session::new(
            SessionId::new("session-2").expect("valid id"),
            ResourceBudget::default(),
        );
        let error = skip_attempt
            .transition(LifecycleState::Closed)
            .expect_err("creating -> closed is not a canonical transition");
        assert!(matches!(error, SessionError::InvalidTransition { .. }));

        // Close path.
        session
            .transition(LifecycleState::Closing)
            .expect("ready -> closing");
        session
            .transition(LifecycleState::Closed)
            .expect("closing -> closed");
        assert!(session.cancellation().is_cancelled());

        // Cancel path: Creating|Ready -> Failed -> Closed, distinct from the
        // graceful close path.
        let mut cancelled = Session::new(
            SessionId::new("session-3").expect("valid id"),
            ResourceBudget::default(),
        );
        cancelled
            .transition(LifecycleState::Ready)
            .expect("creating -> ready");
        cancelled
            .cancel()
            .expect("ready -> failed via explicit cancel");
        assert_eq!(cancelled.state(), LifecycleState::Failed);
        assert!(cancelled.cancellation().is_cancelled());
        cancelled
            .transition(LifecycleState::Closed)
            .expect("failed -> closed");
    }

    #[test]
    fn context_and_page_lifecycle_match_canonical_contract() {
        let mut session = ready_session(ResourceBudget::default());
        let context_id = ContextId::new("context-1").expect("valid id");
        session
            .open_context(context_id.clone())
            .expect("context creation succeeds");
        assert_eq!(
            session
                .context(&context_id)
                .expect("context exists")
                .state(),
            LifecycleState::Ready
        );

        let page_id = PageId::new("page-1").expect("valid id");
        session
            .open_page(&context_id, page_id.clone(), PageResourceBudget::default())
            .expect("page creation succeeds");
        assert_eq!(session.usage().pages, 1);
        {
            let page = session
                .context(&context_id)
                .and_then(|context| context.page(&page_id))
                .expect("page exists");
            assert_eq!(page.state(), LifecycleState::Ready);
            assert_eq!(page.context_id().as_str(), "context-1");
        }

        session
            .close_page(&context_id, &page_id)
            .expect("page close succeeds");
        assert_eq!(session.usage().pages, 0);
        assert!(session
            .context(&context_id)
            .expect("context still exists")
            .page(&page_id)
            .is_none());

        session
            .close_context(&context_id)
            .expect("context close succeeds");
        assert_eq!(session.context_count(), 0);
        assert_eq!(session.usage().contexts, 0);
    }

    #[test]
    fn closing_a_context_with_open_pages_closes_them_and_updates_aggregate_usage() {
        let mut session = ready_session(ResourceBudget::default());
        let context_id = ContextId::new("context-1").expect("valid id");
        session
            .open_context(context_id.clone())
            .expect("context creation succeeds");
        session
            .open_page(
                &context_id,
                PageId::new("page-1").expect("valid id"),
                PageResourceBudget::default(),
            )
            .expect("first page fits");
        session
            .open_page(
                &context_id,
                PageId::new("page-2").expect("valid id"),
                PageResourceBudget::default(),
            )
            .expect("second page fits");
        assert_eq!(session.usage().pages, 2);

        session
            .close_context(&context_id)
            .expect("closing a context closes its open pages instead of leaving them orphaned");
        assert_eq!(
            session.usage().pages,
            0,
            "aggregate page usage must reflect the closed pages"
        );
        assert_eq!(session.context_count(), 0);
    }

    #[test]
    fn cancelling_session_cascades_to_contexts_and_pages() {
        let mut session = ready_session(ResourceBudget::default());
        let context_id = ContextId::new("context-1").expect("valid id");
        session
            .open_context(context_id.clone())
            .expect("context creation succeeds");
        let page_id = PageId::new("page-1").expect("valid id");
        session
            .open_page(&context_id, page_id.clone(), PageResourceBudget::default())
            .expect("page creation succeeds");

        let page_token = session
            .context(&context_id)
            .and_then(|context| context.page(&page_id))
            .expect("page exists")
            .cancellation();
        let context_token = session
            .context(&context_id)
            .expect("context exists")
            .cancellation();

        session
            .cancel()
            .expect("session cancel is a valid transition from ready");

        assert!(session.cancellation().is_cancelled());
        assert!(context_token.is_cancelled());
        assert!(page_token.is_cancelled());
        assert_eq!(
            session
                .context(&context_id)
                .expect("context still tracked")
                .state(),
            LifecycleState::Failed
        );
    }

    #[test]
    fn context_budget_is_a_hard_limit() {
        let mut session = ready_session(ResourceBudget {
            max_contexts: 1,
            max_pages: 8,
        });
        session
            .open_context(ContextId::new("context-1").expect("valid id"))
            .expect("first context fits");
        let error = session
            .open_context(ContextId::new("context-2").expect("valid id"))
            .expect_err("second context exceeds budget");
        assert!(matches!(
            error,
            SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Contexts,
                ..
            }
        ));
    }

    #[test]
    fn aggregate_page_budget_is_a_hard_limit_across_contexts() {
        let mut session = ready_session(ResourceBudget {
            max_contexts: 2,
            max_pages: 1,
        });
        let context_a = ContextId::new("context-a").expect("valid id");
        let context_b = ContextId::new("context-b").expect("valid id");
        session.open_context(context_a.clone()).expect("context a");
        session.open_context(context_b.clone()).expect("context b");
        session
            .open_page(
                &context_a,
                PageId::new("page-1").expect("valid id"),
                PageResourceBudget::default(),
            )
            .expect("first page fits the session-wide budget");
        let error = session
            .open_page(
                &context_b,
                PageId::new("page-2").expect("valid id"),
                PageResourceBudget::default(),
            )
            .expect_err(
                "second page exceeds the aggregate session budget even in a different context",
            );
        assert!(matches!(
            error,
            SessionError::ResourceLimitExceeded {
                kind: ResourceKind::Pages,
                ..
            }
        ));
    }

    #[test]
    fn every_page_resource_category_enforces_a_hard_limit() {
        let mut page = Page::new(
            PageId::new("page-1").expect("valid id"),
            ContextId::new("context-1").expect("valid id"),
            tiny_page_budget(),
        );
        page.transition(LifecycleState::Ready)
            .expect("creating -> ready");

        for kind in [
            PageResourceKind::DomNodes,
            PageResourceKind::Timers,
            PageResourceKind::EventListeners,
            PageResourceKind::NetworkRequests,
            PageResourceKind::Artifacts,
        ] {
            page.account(kind, 1).unwrap_or_else(|error| {
                panic!("first unit of {kind:?} must fit the budget: {error}")
            });
            let error = page
                .account(kind, 1)
                .expect_err("second unit must be rejected by the hard limit");
            assert!(
                matches!(error, SessionError::PageResourceLimitExceeded { kind: rejected, .. } if rejected == kind),
                "unexpected error for {kind:?}: {error:?}"
            );
        }

        // Bytes uses a wider budget (10) to exercise byte-granular accounting.
        page.account(PageResourceKind::NetworkBytes, 10)
            .expect("bytes within budget");
        let error = page
            .account(PageResourceKind::NetworkBytes, 1)
            .expect_err("byte budget is a hard limit");
        assert!(matches!(
            error,
            SessionError::PageResourceLimitExceeded {
                kind: PageResourceKind::NetworkBytes,
                ..
            }
        ));
    }

    #[test]
    fn releasable_categories_free_capacity_and_cumulative_categories_reject_release() {
        let mut page = Page::new(
            PageId::new("page-1").expect("valid id"),
            ContextId::new("context-1").expect("valid id"),
            tiny_page_budget(),
        );
        page.transition(LifecycleState::Ready)
            .expect("creating -> ready");

        page.account(PageResourceKind::DomNodes, 1)
            .expect("one dom node fits");
        assert!(page.account(PageResourceKind::DomNodes, 1).is_err());
        page.release(PageResourceKind::DomNodes, 1)
            .expect("dom nodes are releasable");
        page.account(PageResourceKind::DomNodes, 1)
            .expect("capacity is available again after release");

        page.account(PageResourceKind::NetworkRequests, 1)
            .expect("one request fits");
        let error = page
            .release(PageResourceKind::NetworkRequests, 1)
            .expect_err("network requests are cumulative and cannot be released");
        assert_eq!(
            error,
            SessionError::ResourceNotReleasable(PageResourceKind::NetworkRequests)
        );
    }

    #[test]
    fn resource_accounting_requires_a_ready_page() {
        let mut page = Page::new(
            PageId::new("page-1").expect("valid id"),
            ContextId::new("context-1").expect("valid id"),
            PageResourceBudget::default(),
        );
        let error = page
            .account(PageResourceKind::DomNodes, 1)
            .expect_err("page still in Creating state cannot account resources");
        assert_eq!(error, SessionError::NotReady);

        page.transition(LifecycleState::Ready)
            .expect("creating -> ready");
        page.cancel().expect("ready -> failed via explicit cancel");
        let error = page
            .account(PageResourceKind::DomNodes, 1)
            .expect_err("cancelled page cannot account new resources");
        assert_eq!(error, SessionError::NotReady);
    }

    #[test]
    fn invalid_transition_is_reported_at_every_identity_level() {
        let mut session = Session::new(
            SessionId::new("session-1").expect("valid id"),
            ResourceBudget::default(),
        );
        assert!(matches!(
            session.transition(LifecycleState::Closing).unwrap_err(),
            SessionError::InvalidTransition { .. }
        ));

        let mut context = Context::new(ContextId::new("context-1").expect("valid id"));
        assert!(matches!(
            context.transition(LifecycleState::Closed).unwrap_err(),
            SessionError::InvalidTransition { .. }
        ));

        let mut page = Page::new(
            PageId::new("page-1").expect("valid id"),
            ContextId::new("context-1").expect("valid id"),
            PageResourceBudget::default(),
        );
        assert!(matches!(
            page.transition(LifecycleState::Closing).unwrap_err(),
            SessionError::InvalidTransition { .. }
        ));
    }

    #[test]
    fn closing_a_session_without_closing_children_first_still_reaches_closed() {
        // Mirrors the graceful-close contract used by session.close.v1:
        // callers may close a session directly; contexts/pages are cascaded
        // into a failed/cancelled state rather than left orphaned.
        let mut session = ready_session(ResourceBudget::default());
        let context_id = ContextId::new("context-1").expect("valid id");
        session
            .open_context(context_id.clone())
            .expect("context creation succeeds");
        session
            .open_page(
                &context_id,
                PageId::new("page-1").expect("valid id"),
                PageResourceBudget::default(),
            )
            .expect("page creation succeeds");

        session
            .transition(LifecycleState::Closing)
            .expect("ready -> closing");
        session
            .transition(LifecycleState::Closed)
            .expect("closing -> closed");
        assert_eq!(session.state(), LifecycleState::Closed);
        assert_eq!(
            session
                .context(&context_id)
                .expect("context still tracked")
                .state(),
            LifecycleState::Failed
        );
    }
}
