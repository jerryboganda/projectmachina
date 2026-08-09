# M2-T09 — Navigation lifecycle design: URL loading → parse → tree-build → lifecycle

Status: design only, not implemented. No crate under `crates/` is modified by
this document. Written against the *merged* public APIs of `machina-network`
(`crates/network/src/lib.rs`, M2-T02), `machina-html-tree-builder`
(`crates/html-tree-builder/src/lib.rs`, M2-T04), and `machina-native-core`
(`crates/native-core/src/lib.rs`), plus the pinned schema
`schemas/command-model/v0.1/command-model.json`. `.agent-state/design/M2-T02-http-loader-design.md`
and `.agent-state/design/M2-T04-tree-builder-design.md` are cited for
rationale but the actual merged code is authoritative wherever the two
disagree.

## 0. Scope, dependencies, and the hard blocker this design works around

Milestone doc (`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`, M2-T09)
deliverables:

- Implement the top-level navigation state machine and document replacement.
- Stream the response into the parser, pause for scripts, resume the event
  loop.
- Emit canonical navigation/document lifecycle events and basic current
  URL/history state.

Acceptance criteria: a fixture reaches response → parsing → DOM-interactive →
task-ready/load states in valid order; a superseded navigation cannot satisfy
a later wait; cancel/redirect/script-error produce classified outcomes.

Declared dependencies: M2-T02 (merged), M2-T04 (merged), M2-T07 (V8
bindings/isolate lifecycle — **not implemented**), M2-T08 (event loop, tasks,
microtasks, timers — **not implemented**, in-progress design only). Per
`agents/WORK_QUEUE.md`, M2-T06/T07/T08 are still open; M2-T09 cannot itself
close that gap.

**This design explicitly does not assume a working V8 bridge or event loop.**
Every point in the state machine where a script would execute is modeled as a
named extension seam with a documented interim behavior, not as a call into
code that does not exist yet. Section 4 enumerates exactly what M2-T09 can
ship *today* (URL load → parse → tree-build → a document-lifecycle skeleton
with **no script execution**) versus what it must leave as a seam for T07/T08
to fill in later without another breaking change to this state machine.

Also load-bearing, per `agents/BLOCKERS.md` BLK-004: `machina-network`'s
`[NOW]` scope is deliberately SSRF/DNS-rebinding/decompression-bomb/timeout
defense only. There is **no cookie jar, no CORS, no HTTP cache, no
redirect-credential-policy hardening** anywhere below this task (those are
M3-T06). Navigation must not synthesize any of that behavior itself — no
implicit cookie storage, no implicit cache reads, no invented CORS checks.
`fetch`/XHR-driven sub-resource loading during parsing (M2-T12) is also out of
scope here; M2-T09 only drives the *document* request, not sub-resources
(images/stylesheets/scripts-as-fetches) referenced from the parsed tree.

## 1. What already exists in the pinned contract (nothing new to add)

Per `.agent-state/design/M2-M1-contract-compatibility-checklist.md` finding 3,
`navigation.goto.v1` is already a pinned `CommandKind` (`schemas/command-model/v0.1/command-model.json`,
`crates/command-model/src/generated.rs`) that currently falls through
`LifecycleEngine::execute`'s `_ => Err(DispatchError::unsupported(...))`
catch-all in `crates/native-core/src/lib.rs` (see the
`unsupported_navigation_is_explicit` test, which pins exactly this). Its
payload and the `navigation.lifecycle.v1` event type are also already pinned:

```jsonc
// NavigationGotoPayload (schemas/command-model/v0.1/command-model.json)
{ "url": "https://…", "wait_until": "commit" | "domcontentloaded" | "load" | "networkidle" }
```

```rust
// crates/command-model/src/generated.rs
pub enum EventType { …, NavigationLifecycleV1, … } // "navigation.lifecycle.v1"
```

**Conclusion for the "what CommandKind gets added" question: none.** The
contract checklist's own guidance (item 2: decide explicitly, early, whether
new surface is a schema-additive bus command or purely internal engine state)
was already applied by whoever pinned the schema before M1 closed —
`navigation.goto.v1` and `navigation.lifecycle.v1` are pre-approved,
schema-stable surface. M2-T09's job is to *implement* the existing
`CommandKind::NavigationGotoV1` match arm in `LifecycleEngine::execute`, not
to run the schema-regen chain (`scripts/contracts/generate.mjs` →
`crates/command-model/src/generated.rs` /
`packages/contracts-ts/src/command-model.ts` → `check.mjs`/`roundtrip.mjs`/
`typecheck.mjs` → SCHEMA_VERSION/SHA bump) at all — that chain stays untouched
by this task. If a future review of this design finds a genuinely new field
is needed on `NavigationGotoPayload` (e.g. `referrer`, `post_data`), that is a
schema-additive follow-up, not silently added to the Rust struct out of band.

The one already-reserved extension point worth calling out: `crates/event-loop`'s
design (`.agent-state/design/M2-T08-event-loop-design.md` §1) lists
`NavigationAndParsing` and `HistoryTraversal` as `TaskSource` enum variants
"reserved + queue plumbing now, behavior implemented by later tasks (M2-T09,
M3)". This design assumes that reservation still holds when M2-T08 lands, and
names it as the eventual task-source for driving parse resumption/script
checkpoints from a real lane (§4.4 below) — but does not require M2-T08's
crate to exist to specify the state machine itself.

No result-envelope convention exists yet for `navigation.goto.v1` beyond the
generic pattern `interaction.click.v1` established (`{"schema": "...", "data":
{...}}`, per native-core's module docs). §5 below defines
`navigation.goto.result.v1` following that same convention.

## 2. Navigation state machine

States (engine-internal; not all are wire-visible — see §5 for which ones
become `navigation.lifecycle.v1` events):

```
Idle
  └─(navigation.goto.v1 accepted)─> Requesting
Requesting                          -- machina_network::NetworkClient::fetch() in flight
  ├─(NetworkError::PolicyBlocked/InvalidUrl/DnsResolutionFailed/
  │  ConnectFailed/TlsFailed/DeadlineExceeded/IdleReadTimeout/
  │  TooManyRedirects/RedirectLoop/InvalidRedirect)─> Failed(classified)
  ├─(NetworkError::Cancelled, or a newer navigation superseded this one)─> Cancelled
  └─(ResponseHead received, 2xx/redirect-terminal/4xx/5xx — anything that
     is not itself mid-redirect, since NetworkClient::fetch already drives
     the whole redirect chain internally)─> Committed
Committed                           -- response head accepted; old Document about to be discarded
  └─(document replacement performed — see §3.1)─> Parsing
Parsing                             -- TreeBuilder::feed() driven by streaming ResponseBody chunks
  ├─(TreeBuilderOutcome::ScriptCheckpoint)─> ScriptBlocked
  ├─(TreeBuilderOutcome::NeedsMoreInput, more body available)─> Parsing (loop)
  ├─(ResponseBody stream error mid-read: NetworkError::* other than a clean EOF)─> Failed(classified)
  ├─(cancellation observed between chunks)─> Cancelled
  └─(TreeBuilderOutcome::Done — Token::Eof processed)─> DomComplete
ScriptBlocked                       -- parser paused at a ScriptCheckpoint (design §6 of M2-T04)
  ├─(script executed — see §4.4 seam)─> Parsing (via resume_after_script)
  ├─(TreeBuilderError from resume_after_script — poisoned instance)─> Failed(classified)
  └─(cancellation / superseding navigation observed while blocked)─> Cancelled
DomComplete                         -- Token::Eof processed; open-elements stack empty; no more bytes
  └─(deferred-script queue drained — §4.4 seam, currently a no-op)─> Interactive
Interactive                         -- "DOMContentLoaded"-equivalent milestone
  └─(load-triggering condition — see §3.4)─> Loaded
Loaded                              -- terminal success state; document is paint-ready
Failed(classified)                  -- terminal; carries a CanonicalErrorCode (§3.5)
Cancelled                           -- terminal; superseded/aborted, never resolves any waiter (§3.6)
```

`TreeBuilderOutcome::NeedsMoreInput` is not a wire-visible state — it is the
normal steady-state result of feeding one `ResponseBody` chunk and looping;
it only shows up in this diagram to make clear that "Parsing" is itself a
loop, not a single call.

### 2.1 State ownership

One `NavigationState` value lives per `Page` inside `EngineSession` (see §3.3
for exactly where — `EngineSession` today owns one `Document`/one
`EventTargetRegistry` for the whole session, not yet per-page; this design
specifies the minimal per-navigation bookkeeping needed without assuming the
page-scoped document refactor that is arguably overdue but out of this
task's stated scope). Only one `NavigationState` may be `Requesting` /
`Committed` / `Parsing` / `ScriptBlocked` at a time for a given page — a
second `navigation.goto.v1` while one is in flight supersedes it (§3.6),
it never queues behind it.

## 3. State transition detail

### 3.1 Requesting → Committed: composing `machina-network`

`navigation.goto.v1`'s handler builds a `machina_network::RequestSpec` for
`GET <NavigationGotoPayload.url>` (no request body; POST-driven navigation —
e.g. a submitted `<form method="post">` — is out of scope for this task,
tracked as a future extension of `RequestSpec` construction, not a state
machine change) and calls the session's `NetworkClient::fetch(spec, session_id,
ctx)`. This is a **blocking call from the navigation driver's perspective**
today (`NetworkClient::fetch`'s own doc comment: "the whole engine is
synchronous today... The returned `ResponseBody` still streams"), which is
exactly why M2-T09 does not require M2-T08's event loop to exist to reach
`DomComplete`/`Interactive` — see §4 for precisely what this buys and what it
cannot buy (script execution).

`fetch` already resolves the entire redirect chain server-side (client.rs's
manual redirect loop, re-entering `NetworkPolicy` at every hop) before
returning — so "Requesting" never sub-states through individual redirect
hops from the navigation driver's point of view; it receives one final
`(ResponseHead, ResponseBody)` pair, with `ResponseHead.redirect_chain`
already populated for the `navigation.lifecycle.v1` event's benefit. This
matches `ResponseHead.final_url`/`redirect_chain` fields exactly as they
exist in `crates/network/src/response.rs` today — no change needed there.

On `Err(NetworkError)`, §3.5 classifies it and the state machine moves to
`Failed` without ever reaching `Committed`. Document replacement has not
happened yet, so the page's prior document (if any) is left untouched —
this is deliberate: a failed navigation must never partially clobber the
previous page (matches the acceptance criterion "a superseded navigation
cannot satisfy a later wait", generalized to failed ones too).

**Document commit point**: `Committed` fires the moment `fetch` returns
`Ok((head, body))` — *before* any bytes of the body have been read. This
matches the spec's "navigate" algorithm's create-the-new-Document-before-
parsing-begins ordering and is also the earliest point at which "this
navigation now owns document replacement" can be safely recorded (needed for
§3.6's supersession check to have something concrete to compare against).

### 3.2 Committed → Parsing: document replacement

On entering `Committed`, the navigation driver:

1. Allocates a fresh `machina_dom::Document` and a fresh
   `machina_events::EventTargetRegistry::for_document(&document)` (mirrors
   `EngineSession::new`'s construction exactly).
2. Swaps them into `EngineSession` in place of the previous
   session/page-scoped document + registry. The old `Document` is dropped;
   its listeners/nodes are not migrated (a full page navigation is a hard
   reset, not an incremental DOM diff — matches spec; SPA-style soft
   navigation is explicitly out of scope for M2 entirely).
3. Constructs one `machina_html::Tokenizer` and one
   `machina_html_tree_builder::TreeBuilder::new(scripting_enabled)` for this
   navigation. **`scripting_enabled` is forced `false` for this task** (see
   §4.1) — this is the single concrete design decision that lets M2-T09 ship
   without M2-T07/T08: with scripting disabled, the tree builder's own
   documented behavior (`crates/html-tree-builder/src/lib.rs` module docs;
   `checkpoint.rs`) still unconditionally fires a `ScriptCheckpoint` on every
   `</script>` (T04 "tracks checkpoints, not execute[s]... every `</script>`
   unconditionally produces one" — that is not gated on `scripting_enabled`),
   so the pause/resume contract is still exercised end to end; only the
   *execution* of the script body is skipped (§4.4).

Only after both are constructed does the state move to `Parsing` — matching
`TreeBuilder`'s own contract that it never owns `Document`/`Tokenizer`, only
borrows them per call.

### 3.3 Parsing loop: composing `machina-network` streaming with the T04 checkpoint contract

This is the core integration point the task packet calls out explicitly.
Driving loop (synchronous, per §3.1's "whole engine is synchronous today"):

```
loop {
    match response_body.next_chunk_blocking(&network_client.handle())? {
        None => {
            // clean EOF: hand the tokenizer end-of-stream, not another chunk
            match tree_builder.finish(&mut document, &mut tokenizer)? {
                TreeBuilderOutcome::Done => break DomComplete,
                TreeBuilderOutcome::ScriptCheckpoint(cp) => goto ScriptBlocked(cp),
                TreeBuilderOutcome::NeedsMoreInput => unreachable!(
                    // finish() always terminates in Done or ScriptCheckpoint
                    // per T04's own contract (tokenizer.finish() feeds Eof)
                ),
            }
        }
        Some(chunk) => {
            match tree_builder.feed(&mut document, &mut tokenizer, &chunk)? {
                TreeBuilderOutcome::NeedsMoreInput => continue, // pull the next chunk
                TreeBuilderOutcome::ScriptCheckpoint(cp) => goto ScriptBlocked(cp),
                TreeBuilderOutcome::Done => break DomComplete,
            }
        }
    }
}
```

Composition notes, each pinned by the merged code:

- **Chunk sizing is `machina-network`'s choice, not the navigation driver's.**
  `ResponseBody::next_chunk_blocking` yields "the next data chunk" from
  whatever the underlying decompression/metering pipeline produced — the
  navigation driver must not assume any particular chunk boundary and must
  feed exactly what it receives to `TreeBuilder::feed` as one call (T04's own
  "chunk-equivalence" test guarantee — "same document whole vs. byte-at-a-
  time produces identical tree+diagnostics" — is precisely what makes this
  safe regardless of `machina-network`'s internal chunking).
- **Errors from `ResponseBody` mid-stream are `NetworkError`, not
  `TreeBuilderError`.** A `next_chunk_blocking` failure (deadline, idle-read
  timeout, cancellation, decompression-ratio bomb, oversized body,
  protocol error) aborts the whole navigation via §3.5's classification;
  it never reaches the tree builder as a synthetic Eof, because a
  transport-level failure must not be silently reported as "the document
  ended cleanly." This matters: without this rule a truncated response could
  produce a syntactically "valid" (if incomplete) DOM and a caller waiting on
  `Loaded` would never learn the load actually failed.
- **`TreeBuilderError` from `feed`/`finish`/`resume_after_script` is always
  `Failed`, never retried.** Per T04's design §6 "Resumability contract":
  `Err` is reserved for genuine internal-invariant violations and poisons the
  `TreeBuilder` instance — the navigation driver must discard the whole
  parsing state (not just retry the call), matching T04's own documented
  contract exactly.
- **`ScriptCheckpoint` transitions to `ScriptBlocked` and stops pulling more
  body chunks** until `resume_after_script` is called (§4.4) — this is the
  literal "pause for scripts and resume" deliverable line from the milestone
  doc. Because `ResponseBody` is a genuine byte-stream (not buffered), no
  additional chunk is read from the network while blocked; backpressure is
  free — the socket-level read simply isn't issued again until resume.

### 3.4 DomComplete → Interactive → Loaded

- **Interactive** (`DOMContentLoaded`-equivalent): fires once `Done` is
  reached AND the deferred-script queue (§4.4 seam — always empty in this
  task's shipped scope, since scripting is force-disabled) has been drained.
  With scripting disabled, `DomComplete → Interactive` is therefore an
  unconditional, immediate transition in this task's shipped behavior — but
  it is modeled as a distinct state (not collapsed into `DomComplete`) so a
  future M2-T07/T08-enabled build can insert real deferred-script draining
  here without another state-machine revision.
- **Loaded**: this task defines "load" purely in terms of the document
  parse completing, matching `wait_until: "load"`'s *minimum honest
  semantics* given no sub-resource loading exists yet (M2-T12, not this
  task) and no scripting exists yet (M2-T07/T08). `Interactive → Loaded` is
  therefore also an immediate, unconditional transition in this task's
  shipped scope. This is a **documented, deliberate under-approximation**,
  not a silent one: once M2-T12 lands sub-resource `fetch`/image/stylesheet
  loading, `Loaded` must additionally wait on that resource queue draining
  (matches spec's real `load` event ordering) — flagged here as required
  follow-up scope for whichever task first makes `Loaded` observably
  different from `Interactive`.
- `wait_until: "commit"` resolves at `Committed`; `"domcontentloaded"` at
  `Interactive`; `"load"` at `Loaded`; `"networkidle"` has **no honest
  definition without M2-T12's fetch/XHR surface to observe network activity
  from** — this task treats `"networkidle"` as equivalent to `"load"` and
  records that equivalence in the completion evidence as a deferred-scope
  item for M2-T12, not a silent behavioral promise.

### 3.5 Failure classification

`NetworkError` (crate-local, deliberately not depending on
`machina-command-model` per its own module docs) is mapped by the navigation
driver — the same pattern `native-core` already uses for
`SessionError`/`QueryError`/`EventError` (see `map_session_error`,
`map_query_error`, `map_event_error` in `crates/native-core/src/lib.rs`) —
onto the existing, pinned `CanonicalErrorCode` set (contract checklist
finding 5: M2 should map into existing codes, not invent new ones):

| `NetworkError` variant | `CanonicalErrorCode` | Rationale |
|---|---|---|
| `InvalidUrl` | `INVALID_URL` | exists precisely for this |
| `PolicyBlocked(_)` | `NETWORK_POLICY_BLOCKED` | exists precisely for this |
| `DnsResolutionFailed`, `ConnectFailed`, `TlsFailed`, `ProtocolError`, `Io` | `NAVIGATION_FAILED` | no finer-grained transport code exists; detail preserved in the message |
| `DecompressionFailed`, `BodyTooLarge` | `NAVIGATION_FAILED` | same bucket — a malformed/oversized response is still "navigation failed," not a policy decision |
| `TooManyRedirects`, `RedirectLoop`, `InvalidRedirect` | `NAVIGATION_FAILED` | redirect-shape failures are navigation failures, not policy denials (policy denials during a redirect hop already surface as `PolicyBlocked` per client.rs's "every hop re-enters the full SSRF policy pipeline") |
| `DeadlineExceeded` | `DEADLINE_EXCEEDED` | exists precisely for this |
| `IdleReadTimeout` | `NAVIGATION_FAILED` | no dedicated idle-timeout code; distinct from `DEADLINE_EXCEEDED` in `NetworkError` but the same caller-facing bucket at this layer — recorded, not silently merged into `DEADLINE_EXCEEDED`'s meaning |
| `BudgetExceeded` | `QUOTA_EXCEEDED` | matches `machina-session`'s existing resource-budget mapping pattern |
| `Cancelled` | `COMMAND_CANCELLED` | matches every other cancellation mapping in the codebase |
| `TreeBuilderError::*` (including `Poisoned`) | `NAVIGATION_FAILED` | parser-invariant failures are still "this navigation did not succeed"; no dedicated parse-error code exists and none should be invented per the contract checklist |

Every `Failed` transition produces one `navigation.lifecycle.v1` event
carrying the mapped `CanonicalErrorCode` (§5) and, on the command's own
outcome if this was the command that triggered the failure synchronously,
a `CanonicalError` on the `CommandOutcome` following the existing
`DispatchError::failed(code, message, retryable)` shape used everywhere else
in `native-core`.

### 3.6 Cancellation and supersession

Two distinct triggers collapse to the same `Cancelled` terminal state:

1. **Explicit cancellation** — `CommandContext.cancellation` (or the owning
   `EngineSession::cancellation()`/session cancel cascade, per
   `EngineSession::cancel`) observed true. `machina-network`'s
   `ResponseBody`/connect-phase code already races every I/O wait against
   `ctx.cancellation` at the true poll layer (`DeadlineGuardedBody`,
   `guarded()` in `client.rs`) — cancellation during `Requesting`/mid-`Parsing`
   is therefore already a solved problem at the network layer; the
   navigation driver only needs to also check cancellation between
   `TreeBuilder` calls (cheap, since each call is already a natural
   yield point) and while `ScriptBlocked` waiting on §4.4's seam.
2. **Supersession** — a second `navigation.goto.v1` accepted for the same
   page while an earlier one is still `Requesting`/`Committed`/`Parsing`/
   `ScriptBlocked`. The navigation driver holds one generation counter per
   page (`u64`, incremented on every `navigation.goto.v1` accepted); every
   in-flight navigation closes over the generation it was started with. Any
   state-machine transition first re-checks
   `page.current_generation == my_generation`; a mismatch means a later
   navigation has already claimed the page, and the stale one immediately
   transitions to `Cancelled` **without touching `EngineSession`'s live
   document/registry** (those belong to the newer, still-live navigation —
   an old one racing to `Committed` after a new one already swapped in its
   own `Document` must never clobber it). This is the literal mechanism
   behind the acceptance criterion "a superseded navigation cannot satisfy a
   later wait": any caller awaiting the old navigation's `wait_until`
   condition observes `Cancelled`, never a stale `Committed`/`Loaded`.

`Cancelled` never emits a success-shaped `navigation.lifecycle.v1` payload —
only a `phase: "cancelled"` variant (§5) distinct from `"failed"`, so
consumers can tell "this was superseded/aborted" apart from "this genuinely
failed" without inspecting a `CanonicalErrorCode`.

## 4. What M2-T09 can and cannot ship without M2-T07/T08

### 4.1 Scripting forced off

`TreeBuilder::new(scripting_enabled: bool)` / `TreeBuilder::with_limits`
already takes this flag. This task's `navigation.goto.v1` implementation
always constructs with `scripting_enabled: false`. Per WHATWG HTML, this
changes tokenizer/tree-builder behavior in a few real, spec-defined ways
(e.g. `<noscript>` content is parsed as ordinary markup, not raw text) — it
is not merely cosmetic, and the completion evidence must record it as a
scoped, spec-accurate choice, not an oversight. It is *not* a workaround
invented for this task: it is the same flag real browsers flip when
JavaScript is disabled, reused here for exactly the reason a spec-compliant
disabled-scripting mode exists — content must still parse correctly with no
script engine present.

### 4.2 `ScriptCheckpoint`s still fire — they are just never executed

Because `TreeBuilder` tracks checkpoints unconditionally (§3.3), the pause
points required by the milestone's "pause for scripts and resume" wording
already exist mechanically without a V8 bridge. This task's shipped
behavior at `ScriptBlocked`: **immediately call `resume_after_script` with
no execution and no side effect**, i.e. the "script executed" arrow in §2 is,
for this task, a synchronous no-op stub, not a real script run. This is
observably different from "script actually ran" (no `document.write()`, no
DOM mutation from the script, no synchronous global evaluation before the
next token) but is spec-legal for "scripting disabled" mode, which is
exactly the mode §4.1 puts the parser in — there is no contradiction, and no
caller can observe a difference between "we chose not to run scripts" and
"scripting is disabled," because those are the same thing here.

### 4.3 What is explicitly *not* claimed as done by this design

- No inline/external script body is ever read from the network or executed.
  `ScriptSource::External`'s `src` attribute is never fetched by this task
  (that would itself require the sub-resource fetch surface M2-T12 owns).
- No `document.write()` support — T04's design already reserves the seam
  (`resume_after_script`'s shape allows an `injected: Option<&[u8]>` param
  later) but this task does not use it, since nothing can produce injected
  bytes without script execution.
- No `beforeunload`/`unload` event dispatch on the old document during
  replacement — that requires `machina-events` dispatch through a script
  handler, which requires the same V8 bridge. Document replacement (§3.2)
  is a hard swap with no script-observable teardown of the old document in
  this task's scope.

### 4.4 The seam M2-T07/T08 fill in later

`ScriptBlocked`'s handling is isolated behind one function boundary
(illustrative signature, not literal Rust to be typed here since this is a
design doc, not code):

```
fn run_or_skip_script(checkpoint: ScriptCheckpoint, doc: &mut Document, page_lane: Option<&LaneHandle>) -> ScriptOutcome
```

Today: always returns `ScriptOutcome::Skipped` synchronously (§4.2). Once
M2-T07 (isolate/context lifecycle) and M2-T08 (event loop, whose §1 already
reserves the `NavigationAndParsing` task source for exactly this) exist, this
function's real implementation submits the checkpoint's script body to the
page's `ExecutionLane` (via `LaneHandle::submit_and_wait`, per the M2-T08
design's UserInteraction-source pattern, since parser-blocking script
execution is synchronous-from-the-parser's-perspective by spec) and returns
`ScriptOutcome::Executed` or a `TerminationCause`-derived failure. **This
design's state machine (§2/§3.3) does not change shape when that lands** —
only `run_or_skip_script`'s body does, and `ScriptBlocked → Parsing`'s
trigger changes from "immediately" to "after the lane finishes running the
script (or times out/terminates)." This is the concrete meaning of "design
around [the M2-T07/T08] dependency rather than assuming it": the pause/resume
contract, generation-based supersession, and failure classification are all
already correct and stable in the presence of a real script engine; only the
one function's body is a stub.

## 5. Wire-visible events

`navigation.lifecycle.v1` payload (opaque JSON string per
`EventEnvelope.payload: String` — not itself schema-typed beyond that,
matching how `interaction.click.result.v1` established the
`{"schema": ..., "data": {...}}` convention):

```jsonc
{
  "schema": "navigation.lifecycle.v1",
  "data": {
    "navigation_id": "<generation-scoped opaque id>",
    "url": "<requested url>",
    "final_url": "<ResponseHead.final_url, once known>",
    "phase": "requesting" | "committed" | "dom_complete" | "interactive" | "loaded" | "failed" | "cancelled",
    "redirect_chain": [{"url": "...", "status": 302}, ...],
    "error": { "code": "NAVIGATION_FAILED", "message": "..." } // only when phase == "failed"
  }
}
```

One event per state entry into a *wire-visible* phase (Requesting, Committed,
DomComplete, Interactive, Loaded, Failed, Cancelled — `Parsing`/
`ScriptBlocked` are internal-only and not separately emitted, since a
`Parsing`→`Parsing` self-loop per chunk would be excessive event volume for
no caller-visible benefit; only the terminal-or-milestone phases matter to
an automation caller).

`navigation.goto.v1`'s own `CommandOutcome.result` (matching the
`interaction.click.result.v1` convention `native-core` established):

```jsonc
{"schema": "navigation.goto.result.v1", "data": {"navigation_id": "...", "final_url": "...", "phase_reached": "loaded"}}
```

The command itself resolves (returns from `navigation.goto.v1`'s dispatch)
once the requested `wait_until` phase is reached (§3.4) — it does not block
until `Loaded` unconditionally. A command that resolves at `"commit"` still
lets parsing continue in the background (synchronous today per §3.1's
blocking-call caveat — see §6 below for what that means for cancellation
timing); a later `wait_until: "load"` navigation-status poll (or another
command) can observe later phases via `navigation.lifecycle.v1` events or a
`SessionHealth`-style status query, which this task should expose
symmetrically to how `EngineSession::health()`/`SessionHealth` already work
for session/context/page state (a `navigation` field added to that struct's
per-page view, not a new command).

## 6. Cancellation semantics recap (navigating away mid-load)

Given §3.1's synchronous-blocking-call reality (no M2-T08 event loop yet),
"navigating away mid-load" for this task's shipped scope means: a second
`navigation.goto.v1` for the same page is dispatched from a *different*
in-flight command execution while the first is still running its
request/parse loop on its own call stack. Concretely:

- The generation counter (§3.6) is bumped the instant the second
  `navigation.goto.v1` is *accepted* (validated, before its own `fetch`
  call), not when it completes.
- The first navigation's next generation check (at the next `TreeBuilder`
  call or `next_chunk_blocking` loop iteration) observes the mismatch and
  transitions to `Cancelled`, additionally flipping a page-scoped
  cancellation flag so its in-flight `NetworkClient::fetch`/`ResponseBody`
  read (which is racing `ctx.cancellation` already, per §3.6.1) unblocks
  promptly rather than running to natural completion first.
- Until M2-T08's real multi-lane concurrency exists, this task's
  "concurrent" case is necessarily coarse: two `navigation.goto.v1` calls
  for the same page cannot literally run in true parallel today (no event
  loop to interleave them) — this document specifies the *contract* (never
  let a stale navigation win), and the concrete mechanism is the shared
  `CancellationToken` each navigation's `CommandContext` carries, which is
  already wired end-to-end through every I/O wait in `machina-network`. The
  worst case without a real event loop is a bounded delay (the first
  navigation's current in-flight `next_chunk_blocking`/`TreeBuilder::feed`
  call completing) before the second one can actually start driving the
  page's document — documented as a known limitation to revisit once
  M2-T08's lane scheduler exists, not silently accepted as "already fully
  concurrent."

## 7. Test/fixture strategy (for the implementing task, not executed here)

- Fixture suite driving the full `Requesting → Loaded` path against a local
  test HTTP server (reusing `crates/network/tests/fixture_navigation.rs`'s
  existing test-server pattern) with real HTML bodies, asserting phase order
  exactly matches §2's diagram.
- Redirect fixture: multi-hop redirect chain resolved by `NetworkClient`
  before `Committed` fires once, with `redirect_chain` populated in the
  `navigation.lifecycle.v1` event.
- Rapid-supersede fixture: two `navigation.goto.v1` calls back to back;
  assert the first never emits `loaded`/`interactive` and its `wait_until`
  caller (if any) observes `cancelled`, not a stale success.
- Cancel-mid-parse fixture: cancel `CommandContext` while streaming a large
  body; assert `Cancelled`, not a partial `Loaded`.
- Script-checkpoint-without-execution fixture: HTML containing
  `<script>...</script>` reaches `ScriptCheckpoint`, resumes immediately, and
  the final DOM matches "scripting disabled" parsing semantics exactly (no
  script side effects observed) — directly exercises T04's
  `tests/script_checkpoint.rs` contract from the navigation-driver side.
- Failure-classification fixture: one case per `NetworkError`/
  `TreeBuilderError` row in §3.5's table, asserting the mapped
  `CanonicalErrorCode`.
- Explicitly excluded from this task's fast gate (recorded as deferred, not
  silently skipped, per this repo's established pattern for T04's
  `#script-on` cases): any fixture requiring actual script execution,
  `document.write()`, sub-resource loading, cookies, or CORS.

## 8. Files reviewed

- `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (M2-T09 deliverables/acceptance/fast gate)
- `crates/network/src/lib.rs`, `client.rs`, `response.rs`, `body.rs`, `error.rs` (merged M2-T02 API)
- `.agent-state/design/M2-T02-http-loader-design.md` (context only)
- `crates/html-tree-builder/src/lib.rs`, `builder.rs`, `checkpoint.rs` (merged M2-T04 API)
- `.agent-state/design/M2-T04-tree-builder-design.md` §5/§6 (script-checkpoint contract, driving methods)
- `crates/native-core/src/lib.rs` (`EngineSession`/`LifecycleEngine`/`NativeEngine` lifecycle, existing `navigation.goto.v1` placeholder test)
- `crates/command-bus/src/lib.rs` (`CommandContext`/`CancellationToken`)
- `crates/command-model/src/generated.rs`, `schemas/command-model/v0.1/command-model.json` (pinned `NavigationGotoPayload`, `WaitUntil`, `navigation.lifecycle.v1`)
- `.agent-state/design/M2-M1-contract-compatibility-checklist.md` (guidance on adding `CommandKind`s / result-envelope convention)
- `.agent-state/design/M2-T08-event-loop-design.md` §1, §4, §6 (task-source reservations, cancellation/termination model, network-completion callback shape — cited as the eventual seam, not a dependency)
- `agents/BLOCKERS.md` BLK-004 (deferred network-hardening scope: no cookies/CORS/cache)
- `agents/WORK_QUEUE.md`, `agents/CURRENT_STATE.md` (confirms M2-T06/T07/T08 not yet merged)
- `crates/session/src/lib.rs` (`LifecycleState`)
