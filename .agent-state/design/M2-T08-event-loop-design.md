# M2-T08 Design — Web Event Loop, Tasks, Microtasks, Timers (`crates/event-loop`)

> Produced by a wave-1 architect research agent. Read-only; no files changed.

**Repository-state finding:** `crates/event-loop`, `crates/runtime-v8`, `crates/network` are empty and NOT in root `[workspace].members`. Neither M2-T06 nor M2-T02 has landed code — this design is a forward interface contract so those tasks can implement against a fixed shape.

**Schema gap found:** `architecture/ERROR_MODEL.md` documents `SCRIPT_TERMINATED`/`HEAP_LIMIT_EXCEEDED` but the pinned `CanonicalErrorCode` enum (`schemas/command-model/v0.1/command-model.json` + generated Rust mirror) does NOT contain them yet — real blocker for a fully typed "infinite microtask/timer patterns hit typed budgets" acceptance criterion. Interim mapping specified in §9.

## 1. Task source model / execution lane

**Execution lane** = one `IsolateHandle` + `ContextHandle`(s) + ordered task-source queues + one microtask checkpoint driver — the affinity unit from `EVENT_LOOP_AND_SCHEDULER.md`. Not necessarily a dedicated OS thread; lanes are cooperatively multiplexed onto a bounded worker-thread pool by the fairness scheduler (§5). Mutual exclusion is structural (one thread drives a lane at a time), not lock-based.

```rust
pub struct LaneId(u64); // process-unique, generational
pub struct ExecutionLane { id: LaneId, session_id: SessionId, page_id: PageId,
    isolate: runtime_v8::Isolate, context: runtime_v8::Context,
    sources: TaskSourceQueues, timers: TimerWheel,
    inbox: crossbeam_channel::Receiver<LaneMessage>, budget: LaneBudgetState, clock: Arc<dyn Clock> }
pub struct LaneHandle { id: LaneId, inbox: Sender<LaneMessage>, waker: Arc<LaneWaker>, thread_local_marker: Arc<AtomicU64> }
```
`LaneRegistry` (owned by native-core) maps `PageId -> LaneHandle` — the join point for network completion delivery (§6) and command dispatch.

**Task sources** — enum, not open string, so unsupported sources fail typed:
`Timer, Networking, UserInteraction, PostedMessage, HistoryTraversal, NavigationAndParsing, Lifecycle`. DOM mutation reactions and promise jobs are **not** macrotask sources — they're microtask-class jobs (§2), not scheduled on `TaskSourceQueues`.

Each source's routing to its lane: Timer is lane-local (`TimerWheel`, no cross-thread hop). Networking arrives via `NetworkCompletionSink::deliver` looking up `LaneRegistry` (§6). UserInteraction via `LaneHandle::submit_and_wait(Task, deadline)` with same-thread reentrancy detection (runs inline to avoid self-deadlock) else blocks on a oneshot bounded by `CommandContext.deadline`. PostedMessage/HistoryTraversal/NavigationAndParsing/Lifecycle are reserved variants + queue plumbing now, behavior implemented by later tasks (M2-T09, M3) — stable contract, no later breaking change needed.

## 2. Microtask draining integration with V8

**Required `runtime-v8` interface** (flagged as an M2-T06 ABI requirement, not yet in its task packet): `set_microtask_policy_explicit`, `enqueue_microtask(job) -> Result<(), BridgeError>`, `run_microtasks(budget) -> MicrotaskReport{jobs_run, pending_after, terminated}`, and critically a **cross-thread-safe** `terminate_execution`/`cancel_terminate_execution`/`is_execution_terminating` (V8 itself guarantees this cross-thread; the facade must preserve it — the watchdog in §4.2 is impossible otherwise).

`enqueue_microtask` accepting arbitrary native callbacks means DOM mutation-observer callbacks and custom-element reactions enqueue into the **same underlying V8 microtask queue as promise jobs** — one `MicrotaskCheckpoint` driver, FIFO cross-source ordering for free, matches spec model. `event-loop` doesn't need to distinguish a promise job from a mutation-observer job.

**When checkpoints happen (HTML processing model):**
1. **Reentrancy-tracked checkpoint on JS exit** — every native→JS entry wrapped in a `JsEntryGuard` incrementing a per-lane depth counter; only when it returns to 0 does `run_microtasks(budget)` fire. Nested JS calls don't checkpoint until the outermost call returns (matches spec exactly).
2. **End-of-task checkpoint** — after a `Task`'s handler returns, one more unconditional `perform_microtask_checkpoint()` (cheap no-op if step 1 already drained; catches native-only handlers that call `enqueue_microtask` directly).
3. No checkpoint mid-task except at JS-stack-empty points — preserves synchronous ordering scripts depend on.

`requestAnimationFrame`/rendering-step integration is explicitly NOT added as a stub `TaskSource` in M2-T08 (`NATIVE_ENGINE.md`: no full layout/paint) — reserved for whichever milestone adds it.

**Microtask budget:** `MicrotaskBudget{max_jobs_per_checkpoint, max_wall_time}`. Hitting a limit with jobs still pending is a **typed** runaway condition (`TerminationCause::LaneBudgetExhausted{Microtask}`), not a silent partial drain — never move to the next macrotask with stale microtasks pending (spec forbids it).

## 3. Timer implementation

- `setTimeout`/`setInterval`: delay clamped `>=0`. **Nesting clamp**: `Task.nesting_level` inherited+1 from the currently-running timer callback; `nesting_level > 5` → effective delay `max(delay, 4ms)` (HTML's current threshold).
- **Drift policy**: `next_fire = scheduled_time + interval` (not actual-fire + interval), prevents slow drift. **Catch-up coalescing**: missed interval boundaries during a busy/suspended lane are NOT replayed — `next_fire` advances to `max(scheduled_next, now)`, only one task enqueued for "now" — bounds burst from a starved interval.
- Cancellation by monotonically increasing `TimerHandleId` (never reused while a pending firing references it — avoids ABA).
- `TimerWheel::suspend()`/`resume()` (called on navigation/teardown) stops promotion to the task queue without discarding registrations — a mid-navigation-replace page doesn't leak fires into a soon-to-be-destroyed document.

**Fake-clock testability** — `LoopInstant` is a crate-local `u128`-nanosecond tick (not `std::time::Instant`, which can't be manually advanced); both `WallClock` and `VirtualClock` produce comparable ticks so `TimerWheel` is clock-agnostic. `VirtualClock::advance(Duration)` bumps time with no thread sleep. **Deliberate split**: timer *ordering* uses this fakeable loop clock; **`CommandContext.deadline` always stays real, unfaked `std::time::Instant`** — deadline/cancellation is a safety property that must never be alterable by test config (could mask a real runaway-script bug), while timer fire order is a legitimately-fakeable scheduling property. Test API: `EventLoop::with_virtual_clock()`, `handle.advance(dur)`, `loop.run_until_idle()` — no real sleep in the fast-gate path.

## 4. Cancellation / script-termination propagation

**Two independent budget layers:**
1. **Command-scoped** (`CommandContext.deadline`+`CancellationToken`) — bounds one in-flight command. Checked at task dequeue, checkpoint-batch boundaries, and via the watchdog for non-yielding code.
2. **Lane-scoped ambient budget** (`LaneBudgetState`, no `CommandContext` involved) — bounds background activity continuing *after* a command already returned (unbounded `setInterval` work, a self-rescheduling promise chain). `LaneBudgetConfig{max_task_slice, microtask: MicrotaskBudget, max_consecutive_timer_reentries, hard_wall_ceiling}`.

**Termination watchdog** (handles the non-cooperative case): cooperative checks only catch scripts making calls/backedges — a bare `while(true){}` never reaches one. V8's `TerminateExecution()` is checked at V8's internal interrupt points regardless, and is documented safe to call cross-thread. A lightweight watchdog thread registers `(LaneId, hard_ceiling=min(command_deadline, lane.hard_wall_ceiling), cause)` before running a task/checkpoint, deregisters after (cheap lock-free map op), polls every `watchdog_poll_interval` (prod ~5ms, test-only override ~1ms) and calls `isolate.terminate_execution()` cross-thread the instant a ceiling passes — independent of whether the script ever yields. This is the concrete mechanism for BOTH cooperative runaway patterns (§2's budget) AND non-cooperative ones (this watchdog).

**Termination classification:**
```rust
pub enum TerminationCause { CommandDeadlineExceeded, CommandCancelled, SessionCancelled,
    LaneBudgetExhausted(LaneBudgetKind), HeapLimitExceeded }
```
After termination fires: one bounded final `run_microtasks()` pass flushes/rejects already-queued jobs fast (each aborts under the termination flag), then `cancel_terminate_execution()` **only if** cause was command-scoped (isolate is fine, one call aborted — resumable). For `LaneBudgetExhausted`/`HeapLimitExceeded`/`SessionCancelled`, the lane is marked `Degraded`, isolate NOT resumed — page requires navigation/recreate (prevents silently continuing a runaway page).

Mapping: `CommandDeadlineExceeded`→`DEADLINE_EXCEEDED` (exists) · `CommandCancelled`/`SessionCancelled`→`COMMAND_CANCELLED` (exists) · `LaneBudgetExhausted`→`SCRIPT_TERMINATED` (documented but **missing from generated schema**, §9) · `HeapLimitExceeded`→`HEAP_LIMIT_EXCEEDED` (same gap). If no command is in flight (background storm), surfaces as page-level `script.error.v1` telemetry, not a manufactured command failure.

## 5. Fairness accounting

Two mechanisms mirroring `machina_scheduler::FairQueue`'s existing tenant-rotation pattern, applied in-process:
1. **Weighted round-robin lane multiplexer** — N bounded worker threads pull lanes from a `LaneRunQueue`; rotation visits tenants (sessions) round-robin, higher-priority lanes (interactive/foreground) go first within a tenant.
2. **Bounded task slice + credit accounting** — each thread runs a lane for at most `max_task_slice` wall time (checked between tasks/checkpoint batches, not mid-task) before voluntarily yielding even with ready work left — requeued at the back or deprioritized via `LaneCredits` (rolling-window token bucket) if it's exceeded its session's fair share. This bounds cooperative overuse; the watchdog (§4) bounds non-cooperative overuse (a slice that never yields at all) — together, no single lane can hold a worker thread indefinitely or starve siblings.

## 6. Network completion resumption

**Assumed M2-T02 shape**: streaming `Future`/`Stream`-based internal API driven by its own async executor (composes naturally with cancel-by-drop, backpressure, deadline propagation).

**But the `event-loop`↔`network` boundary is deliberately callback-shaped, not Future-shaped** — so `event-loop`/`runtime-v8` never depend on whatever async runtime `network` picks, and critically, the IO-executor thread never touches V8:
```rust
pub trait NetworkCompletionSink: Send + Sync { fn deliver(&self, page_id: &PageId, event: NetworkCompletion); }
pub enum NetworkCompletion { HeadersReady{..}, BodyChunk{..}, BodyComplete{..}, Failed{..}, Aborted{..} }
```
Flow: network's IO thread calls `sink.deliver(...)` (cheap thread-safe send, V8 never touched here) → sink looks up `page_id` in `LaneRegistry`: **found** → wraps as `LaneMessage::Network`, sends to lane inbox, wakes if parked; **not found** (page navigated away) → **dead letter**, dropped with a `network.completion.orphaned.v1` counter, no error, no panic/UAF risk. Lane driver drains its inbox as a first-class step on its next iteration, converts to `Task{source: Networking}` — **all V8 interaction happens strictly on the owning lane**, honoring single-owning-thread.

## 7. Test strategy → acceptance criteria

| Criterion | Mechanism |
|---|---|
| Task/microtask/timer order | Fixture-driven ordering tests (`__machina_test_mark` binding + marker diff): sync-before-microtask-before-macrotask, `.then` chain interleave vs `queueMicrotask`, MutationObserver-after-promise-jobs-in-same-checkpoint (validates §2's shared-queue design), nested `setTimeout(0)` vs microtasks. |
| Timer order specifically | `VirtualClock`-driven, zero real sleeps: nesting-clamp test (6-deep chain asserts 4ms floor from level 6), drift/coalescing test (lane busy past 3 boundaries → exactly one catch-up fire). |
| Infinite patterns hit typed budgets, not starvation | (a) microtask storm → `LaneBudgetExhausted(Microtask)`; (b) self-rescheduling zero-delay `setInterval` → `LaneBudgetExhausted(Timer)`; (c) non-yielding `while(true){}` under a short deadline → watchdog terminates within `poll_interval+epsilon` (1ms test config); (d) **non-starvation proof**: (a)/(b)/(c) on lane A concurrently with lane B's normal periodic workload on the same thread pool — assert lane B's latency stays within SLA throughout. |
| Network completion resumes on owning lane | Fake `NetworkCompletionSink` from a separate thread delivering events for a live lane, assert resolution executes on the lane's own thread (`thread_local_marker`); second test delivers completion *after* the lane was dropped — assert dead-letter path, no panic, no crash under Miri/sanitizer. |

Fast gate: "ordering and fake-clock tests" = rows 1-2 (deterministic, no wall-clock sleep, stays in `FAST_INNER_LOOP.md`'s 1-8min budget). "cancellation/starvation negative tests" = row 3 with fast test-only watchdog interval. Sanitizer/FFI checks stay owned by M2-T06's own fast gate, not duplicated here.

## 8. Module layout

```
crates/event-loop/  Cargo.toml (add to root [workspace].members)
  src/ lib.rs · clock.rs (Clock trait, LoopInstant, WallClock, VirtualClock) ·
       task.rs · microtask.rs (budget, checkpoint driver, JsEntryGuard) ·
       timers.rs (TimerWheel) · lane.rs (ExecutionLane, LaneHandle, LaneRegistry, submit/submit_and_wait) ·
       scheduler.rs (LaneRunQueue wraps machina_scheduler::FairQueue, LaneCredits) ·
       watchdog.rs (TerminationWatchdog thread) · cancellation.rs (TerminationCause, resume-vs-degrade policy) ·
       network_bridge.rs (NetworkCompletionSink, dead-letter) · errors.rs (§9 interim mapping) ·
       telemetry.rs (trace events via CommandContext::record_trace) ·
       test_support.rs (feature="test-support": marker binding, fixture loader, virtual-clock builders)
tests/ fixtures/event-loop/ordering/*.json ·
       integration/event-loop/{ordering,fake_clock_timers,cancellation_starvation,network_resumption}.rs
```
Dependencies: `machina-command-bus`, `machina-command-model`, `machina-session`, `machina-telemetry`, `machina-scheduler` (reused `FairQueue` directly), `runtime-v8`, `crossbeam-channel`, `dashmap`.

## 9. Prerequisites / open risks

1. **Workspace registration gap** — none of `event-loop`/`runtime-v8`/`network` are in root `[workspace].members`; whoever lands first adds its own, this task adds `"crates/event-loop"`.
2. **Schema gap** — `SCRIPT_TERMINATED`/`HEAP_LIMIT_EXCEEDED` documented in `ERROR_MODEL.md` but absent from the generated `CanonicalErrorCode` enum. Define local `EventLoopError` variants and use an **interim mapping** (`CanonicalErrorCode::CapacityUnavailable`, `retryable=false`, `details.cause="script_terminated"`/`"heap_limit_exceeded"` + a `documentation_ref`) until the schema PR lands — never silently reuse `DEADLINE_EXCEEDED`/`COMMAND_CANCELLED` (misclassifies a safety termination as a plain timeout). Additive-only schema change (new enum values), no new ADR needed per AGENTS.md decision rules, but does need `scripts/contracts/generate.mjs` regeneration + contract-fixture tests.
3. **Required `runtime-v8` additions beyond its own task packet**: `set_microtask_policy_explicit`, `enqueue_microtask`, `run_microtasks`, and cross-thread-safe termination fns — flag for M2-T06's ABI review explicitly, since the watchdog design depends on it.
4. **Required `network` addition**: the `NetworkCompletionSink` push-callback boundary (§6), not a raw Future/Waker handoff — keeps `event-loop`/`runtime-v8` free of any specific async-runtime dependency.
5. **Explicitly deferred, not claimed**: cross-lane `postMessage` semantics beyond queue plumbing, `requestAnimationFrame`, worker/SharedWorker task sources, Service Worker task sources — reserved `TaskSource` slots, no behavior yet, per `NATIVE_ENGINE.md`'s phase ordering (frames/workers land later). State this in the completion report's deferred-scope section.

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (M2-T08+neighbors) · `architecture/EVENT_LOOP_AND_SCHEDULER.md` · `architecture/V8_INTEGRATION.md` · `architecture/NATIVE_ENGINE.md` · `architecture/NETWORK_AND_STORAGE.md` · `architecture/SCHEDULER_AND_ISOLATION.md` · `architecture/ERROR_MODEL.md` · `architecture/REPOSITORY_STRUCTURE.md` · ADR-001, ADR-004 · `crates/command-bus`, `crates/session`, `crates/telemetry`, `crates/scheduler`, `crates/worker-pool`, `crates/native-core` (src/lib.rs each) · `crates/command-model/src/generated.rs` + schema.
