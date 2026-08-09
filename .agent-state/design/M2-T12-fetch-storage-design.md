# M2-T12 Design — Fetch, XHR, Cookies and Web Storage Foundations

> Produced by a design-only research agent per the wave dispatched alongside
> M2-T06 (see `agents/WORK_QUEUE.md`, "Active" section). Read-only; no crate
> files changed. This doc is the forward interface contract M2-T12's
> implementer builds against once M2-T05 (merged) and M2-T08 (in design,
> not yet merged) land.

**Repository-state finding:** `crates/storage` is a `.gitkeep`-only empty
scaffold and NOT in root `[workspace].members`. `crates/event-loop` is the
same (design-only per `.agent-state/design/M2-T08-event-loop-design.md`).
`crates/dom`, `crates/network` are merged; `crates/network` already
forward-declares `RequestPurpose::Fetch` (`crates/network/src/policy.rs`)
and a `NetworkClient::fetch` synchronous facade
(`crates/network/src/client.rs`) M2-T12 composes directly.

**Scope source:** `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`
M2-T12 ("Implement fetch, XHR, cookies and web storage foundations"),
cross-checked against `architecture/NETWORK_AND_STORAGE.md`'s "Web storage"
section, which is authoritative on scope boundary:

> `localStorage`: origin-scoped and persistent only in persistent profiles.
> `sessionStorage`: context/page lifecycle as standards require. IndexedDB
> and Cache Storage: P1, implemented through versioned storage service
> interfaces.

So M2-T12's storage surface is **localStorage + sessionStorage only**.
IndexedDB is explicitly P1/out of scope for this task — a later task, not
this one, per that architecture doc. The task prompt's "IndexedDB-subset"
framing is evaluated below (§3.5) and rejected as in-scope for the same
reason: the milestone doc's own acceptance criteria never mention IndexedDB,
only "set/read cookies and storage" and "storage isolation".

## 1. Task boundary and explicit non-goals

**In scope (per milestone deliverables/acceptance criteria):**
- `fetch()`/`Request`/`Response`/`Headers`/`AbortController`/`AbortSignal`
  bindings.
- An XHR subset (enough for `XMLHttpRequest`-dependent fixtures/WPT subset —
  not full XHR, e.g. no `responseXML` synchronous-mode parity requirement).
- A per-context cookie jar (set/read, attribute parsing) at **P0** level.
- `localStorage`/`sessionStorage` with quotas, origin-scoped, isolated
  per context/profile per `NETWORK_AND_STORAGE.md`.
- Credentials/redirects/origin checks/events "at initial P0 level" — i.e.
  same-origin fetch works fully; cross-origin fetch is either blocked or
  passes through without CORS response filtering (see BLK-004 below), but
  the *decision* to block/allow must still be typed and explicit, not silent.

**Explicitly out of scope, deferred to M3-T06 per BLK-004** (already
recorded in `agents/BLOCKERS.md`, reproduced here so this design doesn't
silently re-scope it back in):
- Full cookie-jar/CORS-credential-aware redirect forwarding.
- Referrer-policy computation across redirects.
- HTTP caching semantics (this task's fetch responses are never served from
  a cache; `Cache-Control`/`ETag` parsing, if present at all, is inert).
- Certificate pinning/strict OCSP, proxy support, HTTP/2 push/HTTP/3 — not
  this task's concern; `machina-network` already handles TLS/connection
  policy underneath `fetch`, unchanged by this task.
- A DNS-rebinding-hardened resolver/cache with tenancy-aware variants —
  unchanged, `machina-network` already owns this (M2-T02, security review).

**Consequence for this task's CORS story:** M2-T12 must not claim spec-true
CORS (preflight, `Access-Control-*` response filtering, opaque responses)
because that is M3-T06's scope. This design's fetch binding therefore
implements a **same-origin-first, fail-closed cross-origin policy**: a
cross-origin `fetch()`/XHR request is evaluated by a `FetchOriginPolicy`
hook (§2.4) that defaults to deny (mirroring `machina-network`'s existing
`DenyPrivateNetworks` fail-closed pattern) unless the caller's security
policy explicitly allows it (e.g. an automation fixture that intentionally
disables the same-origin restriction for a scraping profile). This is a
conservative interim, not a CORS implementation — the acceptance criterion
"cross-origin/isolation/quota negative tests" is satisfiable with a
same-origin-or-explicit-allowlist model without inventing preflight
semantics prematurely.

- **Other non-goals**: IndexedDB, Cache Storage API, Service Worker
  storage/registration (all P1 per `NETWORK_AND_STORAGE.md`), `WebSocket`,
  `EventSource`, `navigator.sendBeacon`, HTTP `fetch` from a Worker/frame
  context (frames/workers land later per the M2-T08 design's §9 note).

## 2. `fetch()` binding surface

### 2.1 Composition with `machina-network`

`machina-network::NetworkClient::fetch(spec: RequestSpec, session_id, ctx:
&CommandContext) -> Result<(ResponseHead, ResponseBody), NetworkError>` is
already synchronous-blocking from its caller's perspective and internally
async (owns its own Tokio runtime, `NetworkClient::handle()` exposes a
`tokio::runtime::Handle` for driving `ResponseBody::next_chunk_blocking`).
M2-T12 does **not** change this shape. It adds a new crate,
`machina-fetch` (module layout §7), that:

1. Owns per-context state `fetch` needs that `machina-network` deliberately
   does not (cookie jar, storage) — `machina-network` intentionally has no
   dependency on `machina-session`/`machina-security-policy`
   (`crates/network/src/lib.rs` doc comment), so cookie/storage/quota
   composition is `machina-fetch`'s job, one layer up, exactly mirroring how
   `RequestBudget` is implemented by an adapter over `Page::account` today
   (`crates/network/src/budget.rs`).
2. Translates a JS `Request` (once V8 bindings exist, M2-T07) into
   `machina_network::RequestSpec`, sets `RequestMeta.purpose =
   RequestPurpose::Fetch` (already a defined variant — `machina-network`
   anticipated this task), and calls `NetworkClient::fetch`.
3. Wraps the returned `(ResponseHead, ResponseBody)` into a JS `Response`
   whose `.body` is a `ReadableStream`-shaped binding pulling
   `ResponseBody::next_chunk_blocking` **from the owning execution lane**
   (§4), never from a background thread touching V8.

### 2.2 Streaming body

`ResponseBody` already forbids "read whole body into `Vec`" as the primary
path (`crates/network/src/body.rs` doc comment) — only
`read_to_end_bounded` is the named, budget-capped exception for small
payloads. `machina-fetch` preserves this discipline:
- `Response.text()`/`.json()` use `read_to_end_bounded(limit, handle)` where
  `limit` is the page's remaining `PageResourceKind::NetworkBytes` budget
  headroom (§3.1) — never an unbounded read.
- `Response.body` (the streaming `ReadableStream` binding) pulls one chunk
  at a time via `next_chunk_blocking`, called from a `Task{source:
  Networking}` handler on the lane (per the M2-T08 network-completion
  design, §6 of that doc) rather than as a blocking call inside JS
  execution — a JS `for await` loop over the stream must not block the
  isolate's thread waiting on network IO; each chunk arrives as a
  `NetworkCompletion::BodyChunk` delivered through
  `NetworkCompletionSink::deliver` and resolves the stream's next-chunk
  promise from the lane's own inbox-drain step.

### 2.3 Abort/timeout

- `AbortController`/`AbortSignal` compose on top of
  `machina_command_bus::CancellationToken` (already threaded through
  `CommandContext` and checked at the body-poll layer,
  `DeadlineGuardedBody` in `crates/network/src/body.rs`). `AbortController
  .abort()` calls `CancellationToken::cancel()` on the token backing the
  in-flight `CommandContext`; the next poll of `DeadlineGuardedBody` (every
  ~25ms per its `cancel_check` interval, or the next frame boundary)
  observes `NetworkError::Cancelled` and the JS binding rejects the
  `fetch()` promise with a typed `AbortError`-equivalent, not a generic
  network failure.
- `fetch(url, { signal })` timeout: `machina-fetch` does not invent a new
  timeout primitive — it derives `CommandContext::with_timeout` (or a
  page-scoped default deadline, whichever is shorter) exactly as
  `machina-network`'s existing per-phase timeouts already require
  (connect/idle-read timeouts are `ClientConfigOptions` fields, unchanged
  by this task).
- Because `NetworkClient::fetch`'s request phase is a *blocking* call from
  the caller's perspective (blocks the calling thread on the client's
  internal Tokio runtime until headers arrive), and the lane driving JS
  execution must never block waiting on network IO (violates M2-T08's
  single-owning-thread/fairness model), `machina-fetch`'s binding does not
  call `NetworkClient::fetch` directly from a lane thread. Instead it
  dispatches the call onto a bounded IO-executor pool (reusing the pattern
  M2-T08 already specifies for the network↔event-loop boundary:
  `NetworkCompletionSink::deliver` from an IO thread, `LaneRegistry`
  lookup, `LaneMessage::Network` handoff) so `fetch()` in JS always returns
  a pending `Promise` immediately and resolves asynchronously via the
  lane's normal task/microtask draining (§4) — never a synchronous stall of
  the isolate.

### 2.4 `FetchOriginPolicy` (P0 origin-check hook)

```rust
pub trait FetchOriginPolicy: Send + Sync {
    /// Called once per fetch() / XHR.send() before any network call.
    /// `initiator` is the requesting document's origin (from M2-T09's
    /// navigation/document-origin model, a dependency this task inherits,
    /// not one it defines); `target` is the request URL's origin.
    fn evaluate(&self, initiator: &Origin, target: &Origin, meta: &FetchMeta) -> FetchDecision;
}
pub enum FetchDecision { Allow, Deny { reason: FetchDenyReason } }
pub enum FetchDenyReason { CrossOriginNotAllowed, MixedContentDowngrade, PolicyRejected(String) }
```
Default implementation `SameOriginOnly` denies any `initiator != target`
origin tuple, matching `NormalizedUrl::origin_tuple()` already defined in
`machina-network` (`crates/network/src/url.rs`). This is deliberately the
narrowest correct default — it never silently permits a cross-origin
request review (M3-T06) hasn't validated yet, and it never invents
CORS-shaped allow/deny semantics prematurely. Composed policies (a
scraping/automation profile that intentionally disables same-origin
restriction) implement the same trait, mirroring `NetworkPolicy`'s
composition pattern in `crates/network/src/policy.rs`.

## 3. Storage backend design

### 3.1 Persistence model: in-memory only for this task, with an explicit disk-backed extension point

**Decision:** `machina-fetch`'s (or a sibling `machina-storage`, see §7)
`localStorage`/`sessionStorage`/cookie-jar implementations are **in-memory
only** in M2-T12, keyed per `(ContextId, Origin)`, with a `StorageBackend`
trait boundary that a *later* task (not this one) can implement against a
disk-backed profile store without changing the JS binding surface.

**Why, against the architecture doc's guidance:**
`architecture/NETWORK_AND_STORAGE.md` says `localStorage` is "persistent
only in persistent profiles," and separately describes "Persistent
profiles" as "encrypted-at-rest, tenant-scoped stores with explicit
lifecycle, quota, locking, and migration... SQLite may back local/self-hosted
profiles." That persistent-profile service does not exist yet in this
repository (`crates/storage` is an empty `.gitkeep` scaffold; no profile
crate, no SQLite dependency, no encryption-at-rest key management is wired
anywhere in the merged M2 crates). Building real on-disk persistence in this
task would mean inventing an encrypted-at-rest profile store as an
undocumented side effect of a fetch/storage task — out of this task's
deliverables ("Implement context cookie jar and local/session storage with
quotas" — quotas and jar/store *behavior*, not a persistence backend
project) and a substantial, separately-reviewable security surface
(encryption key management, on-disk quota enforcement, concurrent-profile
locking) that belongs in its own task with its own security review, exactly
as `NETWORK_AND_STORAGE.md`'s "Persistent profiles" section describes as a
distinct concern from "Web storage."

**What this task does instead:** define `StorageBackend` as a narrow trait
(get/set/remove/clear/keys/quota-check, all synchronous — storage access in
every browser is synchronous from script's perspective, and there is no
inherent IO-latency reason to make it async since the in-memory backend
never blocks) and ship exactly one implementation,
`InMemoryStorageBackend`, that satisfies every acceptance criterion
("fixture scripts can... set/read... storage," "storage isolation pass
focused tests," "quota... failures surface canonical/page errors") without
persistence across process restarts. `localStorage`'s spec-required
persistence-across-sessions behavior is then an **explicit, documented gap**
(not silently approximated) recorded in this task's evidence file and a new
`agents/BLOCKERS.md` entry pointing at whichever future task lands the
persistent-profile service — matching this repository's stated principle
in `NETWORK_AND_STORAGE.md`: "unsupported cache semantics may disable
caching rather than approximate silently" (the same discipline applied to
storage: an in-memory `localStorage` that doesn't survive a restart is
disclosed, not silently claimed as spec-complete).

```rust
pub trait StorageBackend: Send {
    fn get(&self, partition: &StoragePartition, key: &str) -> Option<String>;
    fn set(&mut self, partition: &StoragePartition, key: String, value: String)
        -> Result<(), StorageError>; // StorageError::QuotaExceeded{..} on overflow
    fn remove(&mut self, partition: &StoragePartition, key: &str);
    fn clear(&mut self, partition: &StoragePartition);
    fn keys(&self, partition: &StoragePartition) -> Vec<String>;
    fn bytes_used(&self, partition: &StoragePartition) -> u64;
}
```
A disk-backed `SqliteStorageBackend` (or a profile-service-backed one) is
future work implementing the same trait — no JS-binding-visible change
required when it lands, which is the entire point of the boundary.

### 3.2 Per-origin partitioning

`StoragePartition { context_id: ContextId, origin: Origin, kind:
StorageKind }` where `StorageKind::Local | StorageKind::Session`. Cookie jar
is partitioned identically but additionally keyed by the cookie's own
`(domain, path)` per the standard cookie model, itself scoped inside the
owning `ContextId` — `architecture/NETWORK_AND_STORAGE.md`: "centralized
cookie jar per context/profile." `ContextId` (from `machina-session`,
already merged, `crates/session/src/lib.rs`) is the correct isolation
boundary this task inherits, not one it invents: `Session::open_context`
already models "an isolated storage/cookie partition that owns zero or more
pages" verbatim in its doc comment (`crates/session/src/lib.rs` line 36-37)
— that partition boundary was anticipated by M2-T01 specifically for this
task. `sessionStorage`'s lifecycle is `PageId`-scoped per the WHATWG spec
("top-level browsing context" lifetime) layered inside the same
`ContextId` partition — cleared when the owning page closes, which
`Page::transition(Closed)` already emits a lifecycle hook for.

`Origin` itself is `(scheme, host, port)`, matching
`NormalizedUrl::origin_tuple()`'s existing shape in `machina-network`
(`crates/network/src/url.rs`) — `machina-fetch` reuses that tuple type
rather than defining a second, possibly-divergent origin representation.

### 3.3 Cookie jar

A `CookieJar` per `ContextId`, storing `Cookie { name, value, domain, path,
expires, secure, http_only, same_site, host_only }`. Attribute parsing
follows RFC 6265bis at P0 fidelity (domain/path matching, `Secure`/
`HttpOnly`/`SameSite=Strict|Lax|None` enforcement, `__Secure-`/`__Host-`
prefix validation) — explicitly listed as an acceptance criterion
("Cookie attributes/origin/storage isolation pass focused tests"). Public-
suffix-list-aware domain validation and cross-context/tenant cookie
partitioning beyond context-scoping are BLK-004 deferred items (full
CORS-credential-aware forwarding) — this task's jar enforces attributes and
same-context isolation, not the full credential/redirect-forwarding
pipeline M3-T06 owns.

`Set-Cookie` response headers are parsed by `machina-fetch` after
`NetworkClient::fetch` returns `ResponseHead` (cookie handling lives above
`machina-network`, consistent with that crate's explicit non-dependency on
session/security-policy) and written into the requesting `ContextId`'s
jar; outgoing requests read matching cookies from the jar into the `Cookie`
request header before calling `NetworkClient::fetch`.

### 3.4 Quota model

Each `StoragePartition` (per origin, per `StorageKind`) has a byte quota
(default matching common browser behavior, ~5 MiB per origin for
`localStorage`/`sessionStorage` combined per kind — a configurable
`StorageQuotaConfig`, not a hardcoded magic number, so a profile can tune
it). `StorageBackend::set` computing a would-exceed-quota write returns
`StorageError::QuotaExceeded { partition, limit, requested }` — the JS
binding surfaces this as the spec's `QuotaExceededError` `DOMException`
shape (mapped through whatever `CanonicalErrorCode` M2-T07's binding layer
defines for typed JS exceptions), never a silent truncation or a generic
JS `Error`. This satisfies the acceptance criterion "Quota and network
failures surface canonical/page errors correctly" for the storage half;
the network-failure half is already handled by `NetworkError`'s existing
typed variants flowing back from `NetworkClient::fetch`.

### 3.5 IndexedDB — explicitly rejected as in-scope

The task prompt names "IndexedDB-subset" as a possibility to consider. This
design rejects folding any IndexedDB surface into M2-T12: the milestone
doc's M2-T12 acceptance criteria never mention it, and
`architecture/NETWORK_AND_STORAGE.md` explicitly places "IndexedDB and
Cache Storage" at P1 as a *separate* concern from `localStorage`/
`sessionStorage`, "implemented through versioned storage service
interfaces" — a materially larger surface (transactions, object stores,
indexes, structured-clone value serialization, versioned schema upgrades)
that does not belong bundled into a fetch/cookie/storage-quota task. If a
future task adds IndexedDB, `StorageBackend`'s partitioning model
(`StoragePartition` keyed by `ContextId`/`Origin`) is designed to extend
cleanly to a `StorageKind::IndexedDb` variant without redesigning the
isolation boundary — but no IndexedDB code, trait, or binding is part of
this task's deliverables.

## 4. Interaction with M2-T08's event loop

M2-T08 has not merged; this section is a forward contract against
`.agent-state/design/M2-T08-event-loop-design.md` (the wave-1 design doc),
consistent with how that doc itself was written as a forward contract
against then-unmerged `machina-network`/`runtime-v8`.

- **`fetch()` returns a `Promise` synchronously, resolves asynchronously.**
  The V8 binding (M2-T07-provided wrapper template, consumed here) creates
  a JS `Promise` and a paired native resolver, then hands the actual
  request off to the IO-executor path described in §2.3. It never blocks
  the calling lane thread.
- **Task source:** `machina-fetch` request completion is delivered through
  the event loop's existing `NetworkCompletion`/`NetworkCompletionSink`
  boundary (M2-T08 design §6) — `machina-fetch` does not define a second,
  competing completion-delivery mechanism. `HeadersReady` resolves/updates
  the `Response` object's promise-adjacent state; `BodyChunk`/
  `BodyComplete` feed the streaming body reader (§2.2); `Failed`/`Aborted`
  reject the promise with a typed error. All of these arrive tagged
  `Task{source: Networking}` and are only ever consumed on the page's
  owning lane (`LaneRegistry` lookup by `PageId`), preserving the
  single-owning-thread invariant `machina-events`/`machina-dom` already
  rely on (and the reason `EngineSession` is `!Send`/`!Sync`, BLK-005 —
  `machina-fetch` does not change that; it composes on the same lane model,
  not a separate thread pool that would need `Send` DOM/V8 handles).
- **Microtask/promise ordering:** because `fetch()`'s `Promise` resolves
  from a native completion callback (not from JS calling `resolve()`
  directly), the resolution still enqueues a standard V8 promise-reaction
  microtask — no bypass of the microtask checkpoint model in M2-T08 design
  §2. `.then()`/`await` ordering relative to other promises/microtasks is
  unaffected by fetch being network-backed.
- **Abort during a pending macrotask:** if `AbortController.abort()` fires
  while a `Task{source: Networking}` completion is already queued in the
  lane's inbox but not yet processed, the fetch binding's dead-letter check
  (mirroring §6's "page navigated away" dead-letter path) still applies —
  an aborted request's completion, if it arrives after cancellation was
  observed, resolves to the already-rejected `AbortError` state rather than
  double-settling the promise (settling a `Promise` twice is a no-op per
  spec, but `machina-fetch` still avoids doing unnecessary work after
  abort by checking a `Cancelled` flag before constructing a resolution
  value).
- **Resource budget accounting on the lane:** `Page::account(
  PageResourceKind::NetworkRequests/NetworkBytes, ..)` (§5 below) is called
  synchronously at request-initiation and per-chunk time respectively —
  both of those call sites execute on the owning lane (initiation: when JS
  calls `fetch()`; byte accounting: when a `BodyChunk`'s `Task{source:
  Networking}` is processed on the lane), so no cross-thread mutation of
  `Page`'s resource counters is introduced by this task. `Page` remains
  single-lane-owned exactly as today.
- **Storage access is synchronous and lane-local.** `localStorage`/
  `sessionStorage`/cookie reads and writes from JS never cross a thread
  boundary — `StorageBackend` methods execute inline on the calling lane
  (§3.1's `InMemoryStorageBackend` never blocks), so no event-loop task
  source or microtask interaction is needed for storage itself, only for
  the network-backed half of this task (fetch/XHR).

## 5. Quota/resource-accounting integration with `machina-session`'s `ResourceBudget`

`crates/session`'s `PageResourceKind` already lists `NetworkRequests` and
`NetworkBytes` (and `Artifacts`) as accounted categories with hard limits
(`crates/session/src/lib.rs`), and `crates/network/src/budget.rs`'s doc
comment already states the intended adapter: "`native-core` supplies an
adapter over `Session` (calling `Page::account(PageResourceKind::
NetworkRequests, 1)` / `Page::account(PageResourceKind::NetworkBytes, n)`."
M2-T12 does not add a new resource category to `machina-session` for
network usage — that plumbing point already exists and this task's
`fetch()`/XHR binding is simply a second caller of the same
`RequestBudget` adapter `machina-network`'s `NetworkClient` already accepts
(`Arc<dyn RequestBudget>` constructor argument), reused verbatim rather
than reimplemented.

**What is genuinely new for this task:** `PageResourceKind` has no
existing category for storage bytes or cookie-jar size. Two options were
considered:

1. Extend `PageResourceKind` with `StorageBytes` (and reuse `Artifacts`-like
   cumulative semantics, or make it releasable since storage can legitimately
   shrink via `removeItem`/`clear`).
2. Keep storage quota entirely inside `machina-fetch`'s own
   `StorageQuotaConfig` (§3.4), independent of `machina-session`'s
   `PageResourceBudget`.

**Decision: option 1**, extending `PageResourceKind` with a new
`StorageBytes` variant, for consistency with the existing "every page
resource category has accounting and hard-limit behavior" acceptance
criterion from M2-T01 (`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`)
— storage usage is exactly the kind of per-page bounded resource that
model was built for, and per-origin quota (§3.4) composes on *top* of the
page-level `StorageBytes` ceiling as a second, narrower check (an origin
cannot exceed its own quota even if the page's aggregate ceiling has
headroom; the page's aggregate ceiling bounds total storage across every
origin/context a page's script touches indirectly, which per-origin quota
alone does not). `StorageBytes` is releasable (`is_releasable() == true`)
since `removeItem`/`clear` genuinely free capacity, unlike
`NetworkBytes`/`NetworkRequests`/`Artifacts`, which stay cumulative. This
is an additive, backward-compatible change to `crates/session` (a new enum
variant plus its `get`/`set`/limit-lookup match arms and a new
`PageResourceBudget.max_storage_bytes` field with a sane default) — flagged
here as a **required `machina-session` change this task must make**, not an
`event-loop`-shaped forward-only contract, since `machina-session` is
already merged and this task genuinely needs to touch it. This should be
called out explicitly in the implementing PR description (owned path:
`crates/session/src/lib.rs`, in addition to the new `machina-fetch`/
`machina-storage` crate) since M2-T12's nominal owned-path is fetch/storage,
not session — the addition here is narrow (one enum variant, one budget
field, matching existing patterns) and should not be treated as a
license to change unrelated `machina-session` behavior.

Cookie-jar size is accounted the same way (bytes of serialized
name+value+attributes count against `StorageBytes`, or a sibling
`CookieBytes` variant if the implementer finds mixing cookie/storage
budgets under one counter makes negative-test assertions ambiguous — left
as an implementation-time choice, not fixed here, since either satisfies
the acceptance criteria as written).

## 6. XHR subset

`XMLHttpRequest` is implemented as a synchronous-looking JS API backed by
the same async `fetch` machinery underneath (matches how real browsers
implement it today) — `xhr.send()` in async mode (`xhr.open(method, url,
true)`, the only mode this task supports per "XHR subset") drives the same
`NetworkClient::fetch` + lane-task completion path as §2/§4, firing
`readystatechange`/`load`/`error`/`abort`/`timeout` events through
`machina-events`'s already-merged `EventTarget` dispatch (M2-T11, merged)
rather than inventing a second event-dispatch mechanism. Synchronous XHR
(`async=false`) is **out of scope** — it requires blocking the calling
lane's thread on network IO, which directly violates the single-owning-
thread/no-blocking-the-lane model this whole design (§2.3, §4) is built to
avoid, and is deprecated/discouraged in every modern browser besides. This
exclusion should be stated explicitly in the implementing task's evidence
file as a deliberate, documented gap.

## 7. Module layout

```
crates/fetch/                    (new; add to root [workspace].members)
  Cargo.toml
  src/
    lib.rs                       -- crate doc, non-goals, re-exports
    request.rs                   -- JS-facing Request/RequestInit -> RequestSpec translation
    response.rs                  -- Response/Headers, streaming-body reader binding
    abort.rs                     -- AbortController/AbortSignal over CancellationToken
    origin_policy.rs             -- FetchOriginPolicy, SameOriginOnly, FetchDecision
    xhr.rs                       -- XMLHttpRequest subset (async-only), event wiring
    cookie/
      mod.rs                     -- CookieJar, per-ContextId partitioning
      attributes.rs              -- RFC 6265bis attribute parse/match
    storage/
      mod.rs                     -- StorageBackend trait, StoragePartition, StorageQuotaConfig
      memory.rs                  -- InMemoryStorageBackend (only impl this task ships)
      errors.rs                  -- StorageError (QuotaExceeded, etc.)
    errors.rs                    -- FetchError, typed mapping notes for M2-T07's JS-exception layer
  tests/
    fixtures/fetch/*.json        -- fetch/abort JSON+text fixtures
    fixtures/cookie/*.json       -- attribute/isolation fixtures
    fixtures/storage/*.json      -- quota/isolation fixtures
    integration/{fetch,cookies,storage,cross_origin,quota}.rs
```
Naming note: `crates/storage` already exists as an empty scaffold in the
repository tree with a different apparent intent (`architecture/
NETWORK_AND_STORAGE.md`'s heavier "Persistent profiles" concept, encrypted-
at-rest/SQLite-backed). This design deliberately does **not** claim that
scaffold for M2-T12's in-memory web-storage implementation, to avoid
pre-committing that empty crate's eventual name/shape to a scope this task
explicitly excludes (§3.1). The implementer should either add a new
`crates/fetch` crate (recommended, matches this doc) or, if reusing
`crates/storage` is preferred for naming symmetry with
`NETWORK_AND_STORAGE.md`, must record that decision and its scope
boundary explicitly in the task's evidence file so a future persistent-
profile task does not inherit an unexpected in-memory-only implementation
under a name that architecture docs describe as encrypted-at-rest.

Dependencies: `machina-network` (already merged), `machina-session`
(already merged, plus the additive `StorageBytes` change from §5),
`machina-command-bus` (`CancellationToken`, `CommandContext`),
`machina-events` (M2-T11, merged, for XHR event dispatch),
`machina-dom` (M2-T05, merged, only if `Response`/`Request` bindings need
DOM-adjacent types — otherwise no direct dependency), and forward
dependencies on `machina-event-loop` (M2-T08, for `NetworkCompletionSink`/
`LaneRegistry`) and `runtime-v8`/the V8 binding layer (M2-T07) once those
land — this task cannot fully merge before M2-T08 for exactly that reason,
matching `agents/WORK_QUEUE.md`'s recorded dependency ("M2-T12 (needs
T05, T08)").

## 8. Test strategy → acceptance criteria

| Criterion | Mechanism |
|---|---|
| Fixture scripts can fetch JSON/text, abort, set/read cookies and storage | `tests/fixtures/fetch/*.json` against a local fixture HTTP server (reusing `crates/network/tests/support/fixture_process.rs`'s pattern); abort test asserts `AbortController.abort()` rejects with typed error before/after headers arrive. |
| Cookie attributes/origin/storage isolation pass focused tests | RFC 6265bis attribute matrix (domain/path/Secure/HttpOnly/SameSite/prefix); two `ContextId`s never see each other's cookies/storage; two same-context, different-origin pages never see each other's `localStorage`/`sessionStorage`. |
| Quota and network failures surface canonical/page errors correctly | `StorageError::QuotaExceeded` mapped to typed JS exception; oversized `NetworkBytes`/`NetworkRequests` already produce `SessionError::PageResourceLimitExceeded` today (M2-T01, unchanged) — assert both paths surface distinct, typed errors, not one generic failure. |
| Run fetch/cookie/storage WPT subset | Selected WPT `fetch/`, `cookies/`, `webstorage/` tests filtered to same-origin + explicit-allowlist cross-origin cases (cross-origin CORS-filtering WPT cases are out of scope per §1 and must be explicitly excluded from the run list, not silently skipped without a record). |
| Run cross-origin/isolation/quota negative tests | `FetchOriginPolicy::SameOriginOnly` denial fixtures; two-context storage/cookie isolation fixtures; per-origin and page-aggregate `StorageBytes` quota-exceeded fixtures. |

Fast gate: fixture-driven, deterministic, no real external network (reuses
the fixture-process pattern `machina-network`'s own tests already
establish) — stays within `FAST_INNER_LOOP.md`'s budget. WPT subset run is
a separate, explicitly-scoped step per the milestone doc's own fast-gate
line ("Run fetch/cookie/storage WPT subset" is listed as its own item,
distinct from the negative-test fast gate).

## 9. Prerequisites / open risks

1. **Hard dependency on M2-T08 and M2-T07 landing first** — this task
   cannot merge a working `fetch()`/XHR JS binding before the event loop
   (task source, lane, `NetworkCompletionSink`) and the V8 binding scaffold
   exist. The storage/cookie-jar Rust-level logic (§3, independent of any
   JS binding) and the `machina-network` composition (§2.1) can be built
   and unit-tested against `machina-network`/`machina-session` alone ahead
   of that, but full acceptance-criteria satisfaction ("fixture scripts
   can fetch...") needs both upstream tasks merged.
2. **`machina-session` additive change required** (§5) —
   `PageResourceKind::StorageBytes` plus a `PageResourceBudget.
   max_storage_bytes` field. Narrow, backward-compatible, but touches a
   crate outside this task's nominal owned path; must be called out
   explicitly in the implementing PR, not silently bundled.
3. **CORS scope boundary must be re-confirmed against M3-T06 before that
   task starts** — `FetchOriginPolicy::SameOriginOnly` (§2.4) is this
   task's entire cross-origin story; M3-T06 replacing it with real
   preflight/response-filtering semantics is expected and should not be
   treated as fixing a defect in this task's design, only as completing
   deliberately deferred scope (BLK-004).
4. **`crates/storage` naming collision** (§7) — flagged for the
   implementer to resolve explicitly (new `crates/fetch` recommended) so a
   later persistent-profile task does not inherit a misleading crate name.
5. **Synchronous XHR is out of scope** (§6) — must be stated as a
   deliberate, documented gap in the implementing task's evidence file,
   not silently omitted.
6. **IndexedDB is out of scope** (§3.5) — must not be added under cover of
   this task even as a "subset," per the milestone doc's own scope
   boundary; any pressure to include it should be redirected to a new,
   separately tracked task.

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (M2-T12 + neighbors)
· `architecture/NETWORK_AND_STORAGE.md` · `agents/BLOCKERS.md` (BLK-004,
BLK-005) · `agents/WORK_QUEUE.md` · `.agent-state/design/
M2-T08-event-loop-design.md` · `crates/network/src/lib.rs`,
`client.rs`, `body.rs`, `budget.rs`, `policy.rs`, `error.rs`, `url.rs` ·
`crates/session/src/lib.rs` · `crates/dom/src/lib.rs` (leaf crate, no
direct fetch/storage dependency found — confirmed no DOM-layer change is
implied by this task beyond whatever M2-T07's binding wrappers add) ·
`crates/event-loop`, `crates/storage` (both empty `.gitkeep` scaffolds,
confirmed via directory listing).
