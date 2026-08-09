# M2-T06 Design: Narrow C++ V8 Bridge and Safe Rust Facade

> Produced by a wave-1 architect research agent. Read-only; no files changed.
> Pair with `.agent-state/design/M2-T06-security-review.md` before implementing.

Scope: `cpp/v8-bridge/` (existing bootstrap ABI — extend, don't replace) + `crates/runtime-v8/` (currently `.gitkeep` only, not a workspace member).

## 1. V8 acquisition — already decided, don't reopen it

`toolchains/versions.toml` already pins `v8 = "13.1.201.12"` (source `chromium.googlesource.com/v8/v8.git`), `chromium = "131.0.6778.85"`, `acquisition.policy = "external-runtime-only"`, `clean_room = true`. `toolchains/PROVENANCE.md` requires immutable source revision + checksum/signature + build flags + provenance artifact.

- **Build from source** from the pinned tag, via depot_tools + gclient + GN + Ninja (V8's own build system — CMake only builds the bridge). Resolve to an immutable commit SHA; record SHA + DEPS-lock digest + source-tree SHA-256 + exact GN args in `toolchains/versions.toml` or a sibling `toolchains/V8_PROVENANCE.md`.
- Produce **two artifact sets per platform** from the same commit: release/optimized `libv8_monolith`, and an `is_asan=true`/`is_ubsan=true` instrumented build (needed for the sanitizer fast gate).
- **Not a vendored prebuilt binary** — can't get the checksum/build-flags/provenance tuple PROVENANCE.md requires from someone else's binary.
- **Not `rusty_v8`/crates.io `v8`** — that crate exposes V8's full API-shaped Rust surface directly, conflicting with ADR-002's "small audited C ABI," and links a binary Machina doesn't control/audit.
- Licensing: V8 is BSD-3-Clause, compatible with workspace Apache-2.0; still needs an SBOM entry per `SUPPLY_CHAIN.md`.
- No snapshot exists yet at T06 (that's M2-T07) — only obligation now is `machina_v8_bridge_abi_version()`/a new `machina_v8_bridge_v8_version()` checked against `toolchains/versions.toml` at Rust build time, so an accidental V8 bump can't silently ship.

## 2. Narrow C ABI shape (extends the existing bootstrap header)

Opaque handles only — never dereferenced by Rust, never leaked as long-lived V8 references; results cross as owned, copied data.

```c
typedef struct MachinaV8Platform MachinaV8Platform;   // process singleton
typedef struct MachinaV8Isolate  MachinaV8Isolate;    // single owning OS thread
typedef struct MachinaV8Context  MachinaV8Context;    // owned by one Isolate

typedef enum { OK=0, INVALID_ARGUMENT=1, ALREADY_INITIALIZED=2, NOT_INITIALIZED=3,
               WRONG_THREAD=4, STALE_HANDLE=5, ALLOCATION_FAILED=6, INTERNAL_ERROR=7 } MachinaV8Status;

typedef struct { uint64_t max_heap_bytes; uint64_t max_young_gen_bytes; const char *description; } MachinaV8IsolateConfig;

typedef enum { OUTCOME_OK=0, OUTCOME_EXCEPTION=1, OUTCOME_TERMINATED=2, OUTCOME_INTERNAL_ERROR=3 } MachinaV8Outcome;

typedef struct {
  MachinaV8Outcome outcome;
  char *result_json;        // OK only; owned, freed via machina_v8_execute_result_free
  char *exception_kind, *exception_message, *exception_stack, *source_location; // EXCEPTION only
  int32_t is_promise_rejection; // reserved for M2-T08; always 0 in T06
} MachinaV8ExecuteResult;

// machina_v8_bridge_abi_version / _v8_version
// machina_v8_platform_init / _shutdown
// machina_v8_isolate_create / _thread_id / _dispose
// machina_v8_isolate_terminate_execution / _cancel_terminate_execution  <- the ONE pair safe cross-thread
// machina_v8_context_create / _dispose
// machina_v8_context_execute(context, source, len, source_name, MachinaV8ExecuteResult* out)
// machina_v8_execute_result_free
```

**Exception/panic containment (mandatory, ADR-002/V8_INTEGRATION.md: "C++ exceptions do not cross the ABI"):**
- Every exported fn wraps its body in `try{...}catch(...){return INTERNAL_ERROR;}`; `void`-returning functions (`platform_shutdown`/`_dispose`) are specified noexcept-by-construction instead (no heap alloc, no STL beyond raw V8 calls).
- JS exceptions captured via `v8::TryCatch` around `Compile`/`Run`, translated into `MachinaV8ExecuteResult` fields (`HasTerminated()`→`TERMINATED`; `HasCaught()`→`EXCEPTION` with kind/message/stack/location via `Exception::ToString`/`Message::Get*`/`StackTrace::CurrentStackTrace`).
- `std::bad_alloc`/`std::length_error` from the bridge's own glue caught by the same outer `catch(...)`.
- No raw pointer escapes: results are JSON-serialized (bounded) into a heap `char*` owned until `machina_v8_execute_result_free`. `ScriptHandle`/`ValueHandle` (live V8 refs) explicitly out of scope for T06.
- **Thread identity check** on every function taking `Isolate*`/`Context*` except the two `terminate_execution` fns: compares `std::this_thread::get_id()` against the ID captured at `isolate_create` time → `WRONG_THREAD` on mismatch. `context_execute` also compares the context's stored owning-isolate pointer → `STALE_HANDLE` on mismatch. This is defense-in-depth independent of the Rust-side generation check below.

## 3. Safe Rust facade

- `Platform` — process-wide singleton behind `OnceLock`; shutdown guarded by an atomic isolate-count check (errors, not UB, if isolates still live).
- `Isolate` — wraps `NonNull<sys::MachinaV8Isolate>`, **deliberately `!Send`/`!Sync`** (type system, not just runtime check, prevents cross-thread use in safe code). `Drop` calls `machina_v8_isolate_dispose`, debug-asserts current thread == owning thread.
- `Context<'iso>` — wraps `NonNull<...>` + `PhantomData<&'iso Isolate>` + a copy of the isolate's monotonic `u64` id. `Isolate::create_context(&self) -> Result<Context<'_>, BridgeError>` ties context lifetime to the isolate borrow — **borrow checker guarantees `Context` drops before `Isolate` can**. Stored isolate id re-checked on every `execute()` call (Rust-side counterpart to the C++ pointer check). Also `!Send`/`!Sync`.

**Thread-safety vs. M2-T01's async engine session model:** Today's `EngineAdapter::execute` (`crates/command-bus`) is synchronous/blocking, taking `&CommandContext` (deadline + `CancellationToken`) — no `async fn` in the command bus yet. Facade works under that today, composes unchanged if it later goes async:
- `Isolate`/`Context` never leave their OS thread — a dedicated **lane thread**, one isolate per lane thread (placement is a `SCHEDULER_AND_ISOLATION.md` decision, not a runtime-v8 concern).
- Facade exposes a `Send + Sync` **`V8LaneHandle`**: `Arc`-cloneable wrapper around `std::sync::mpsc::SyncSender<LaneRequest>` (deliberately not tokio-coupled). `LaneRequest { script, source_name, deadline, reply: oneshot::Sender<ExecuteOutcome> }`.
- Caller does `lane.send(request)` then `reply.recv_timeout(remaining_budget)` (or `.await`s a `tokio::sync::oneshot` once `EngineAdapter` goes async — lane-thread side unaffected either way).
- Cancellation: lane handle checks `CommandContext::cancellation` before enqueueing; on deadline/cancellation calls `machina_v8_isolate_terminate_execution` (the one cross-thread-safe FFI fn) from the watchdog thread. In-flight `execute()` returns `Terminated` promptly; lane thread stays alive/reusable unless termination doesn't unwind within a grace period, in which case the lane is abandoned (never force-killed) and telemetry marks the session/worker for recycling.

```
crates/runtime-v8/src/
  lib.rs      // Platform, Isolate, Context, V8LaneHandle, BridgeError, ExecuteOutcome
  sys.rs      // private bindgen output
  error.rs    // BridgeError + mapping toward CanonicalErrorCode
  platform.rs / isolate.rs / context.rs   // per above
  lane.rs     // Send+Sync actor: lane thread, channel, termination wiring
```

`ExecuteOutcome::{Ok(Value), Exception{..}, Terminated, InternalError}` — `Exception` does NOT auto-map to `DispatchError` (V8_INTEGRATION.md: "page exceptions are events unless the contract makes them fatal"; `EventType::ScriptErrorV1` already exists in `command-model`). Only `Terminated`/`InternalError` map to `CanonicalErrorCode::{DeadlineExceeded, CommandCancelled, CapacityUnavailable}` at the call site — deciding how `Exception` becomes an event is deferred to whichever task wires this into `native-core`.

## 4. Sanitizer build integration

- New CMake option `MACHINA_SANITIZER = none|asan|ubsan|asan+ubsan` on `cpp/v8-bridge/CMakeLists.txt`, applying `-fsanitize=address,undefined -fno-omit-frame-pointer -g` (Clang 21.1.6 pinned) alongside the existing `-Wall -Wextra -Wpedantic -Werror`/`/W4 /WX`.
- **The linked V8 static lib must be built with matching instrumentation** — sanitized bridge + non-sanitized `libv8_monolith` = false positives/negatives at every allocator boundary. Hence two V8 artifact sets from §1.
- Sanitizer job runs **Linux + Clang only** (MSVC ASan is immature for this) — documented platform limitation, not a gap; normal MSVC/Windows builds still run for functional correctness.
- Rust `-Zsanitizer=address` is nightly-only → scheduled, not fast-gate; fast-gate obligation satisfied by the C++ ASan/UBSan build + Rust tests linked against that sanitized `.a`.
- Required CI job `bridge-sanitizer` (any PR touching `cpp/v8-bridge/**` or `crates/runtime-v8/**`): restore/build the pinned asan+ubsan V8 artifact (cache key = commit SHA + GN-args hash) → `cmake -DMACHINA_SANITIZER=asan+ubsan` build → run a C++-only bridge test binary directly under `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 UBSAN_OPTIONS=halt_on_error=1` → build `crates/runtime-v8` linking the same sanitized `.a`, run full Rust suite under the same env.
- TSan deferred to M8/M9 heavy campaign (per `quality/FUZZING.md` cadence), not dropped — isolate-per-thread-by-construction makes races unlikely but that claim should eventually be TSan-verified.

## 5. Build system plan

**Recommend: `cmake` crate (drive existing CMake) + `bindgen` (raw `extern "C"` layer) + hand-written safe wrapper.**
- **Not `cxx`** — built for idiomatic C++ types/templates, the opposite of what ADR-002 wants (C++ stays entirely inside `cpp/v8-bridge`); would re-widen the boundary.
- **Not hand-rolled `cc::Build`** — CMake already centralizes warnings-as-errors + sanitizer flags per-platform; duplicating in `build.rs` creates two sources of truth.
- **`bindgen`** for the mechanical FFI layer (the header is plain C, no V8 C++ templates to parse) — low risk. `build.rs` runs bindgen + asserts `sys::machina_v8_bridge_abi_version()`/`_v8_version()` match compile-time constants from `toolchains/versions.toml`.
- **Hand-written safety layer** (ownership/Drop/`!Send`/`!Sync`/generation checks/channel actor) — nothing safety-critical is auto-generated; this is the audited part ADR-002 calls for.

```
cpp/v8-bridge/  CMakeLists.txt (extended: MACHINA_V8_ROOT requirement, MACHINA_SANITIZER option)
                include/machina_v8_bridge.h (extended per §2)
                src/machina_v8_bridge.cpp (extended)
                test/  (new: C++-only bridge tests run directly under ASan/UBSan)
crates/runtime-v8/  Cargo.toml (new; add to root workspace members)
                    build.rs (cmake crate + bindgen + version-check codegen)
                    src/{lib,sys,error,platform,isolate,context,lane}.rs
                    tests/{lifecycle,cross_thread,exceptions}.rs
```

`MACHINA_V8_ROOT` is a new required CMake cache var (root CMake currently only builds the bootstrap stub, no real V8 linkage) — configure fails loudly if unset. Note: wiring `native-core` to actually *call* `runtime-v8` for a `script.execute.v1` command is explicitly NOT this task's scope (belongs to M2-T08/T09).

## 6. Test strategy mapped to acceptance criteria

| Criterion | Tests |
|---|---|
| "Simple scripts execute, exceptions return typed diagnostics" | `execute_ok_returns_json_result`, `execute_throw_returns_typed_exception`, `execute_syntax_error_returns_typed_exception`. Promise-rejection field wired but not guaranteed populated in T06 (needs microtask draining — M2-T08); caveat recorded explicitly. |
| "Cross-thread/stale handle operations fail in checked tests" | `trybuild` compile-fail proving `Isolate`/`Context` are `!Send`; `stale_context_after_isolate_dispose` (via `#[cfg(test)]` unsafe escape hatch) → `BridgeError::StaleHandle`, run under ASan; `cross_isolate_context_use` (C++-side pointer check); `foreign_thread_execute_is_rejected` (C++-side thread-id check when the type-system guard is deliberately bypassed). |
| "Isolate/context repeated lifecycle and termination tests" | `repeated_isolate_create_dispose_1000x`, `repeated_context_create_dispose_in_one_isolate` (leak smoke; authoritative leak detection is LSan in the sanitizer job); `infinite_loop_script_terminates_on_deadline` (asserts `Terminated` within deadline+grace AND context is reusable afterward); cross-thread cancellation-token variant; `platform_shutdown_rejected_while_isolates_live` → typed error not panic. |
| "Bridge unit tests under sanitizer build" | All of the above re-run under `MACHINA_SANITIZER=asan+ubsan`, plus the C++-only test binary run directly. |

## 7. Explicit non-goals for M2-T06 (deferred)

Startup snapshot generation/loading + `SnapshotHandle` (→ M2-T07) · Document/Node/Element JS bindings + any live `ValueHandle` exposure (→ M2-T07, depends on M2-T05's DOM handles) · event-loop/microtask/timer integration (→ M2-T08) · wiring into `native-core`'s `EngineAdapter::execute` for a real `script.execute.v1` command (later task) · the V8 build-from-source automation itself is IN scope (nothing under `scripts/build/` for V8 exists yet — T06 must include a documented, reproducible fetch+build recipe since "sanitizer build and ABI/version checks" is literally in its deliverables and needs it).

## Files reviewed

`AGENTS.md` · `architecture/ADR/ADR-001-HYBRID_ENGINE.md` · `architecture/ADR/ADR-002-RUST_V8.md` · `architecture/V8_INTEGRATION.md` · `architecture/NATIVE_ENGINE.md` · `architecture/EVENT_LOOP_AND_SCHEDULER.md` · `architecture/SCHEDULER_AND_ISOLATION.md` · `architecture/REPOSITORY_STRUCTURE.md` · `architecture/boundary-policy.json` · `research/TECHNOLOGY_SELECTION.md` · `toolchains/PROVENANCE.md` · `toolchains/versions.toml` · `security/SUPPLY_CHAIN.md` · `quality/FAST_INNER_LOOP.md` · `quality/FUZZING.md` · `CMakeLists.txt` · `cpp/v8-bridge/CMakeLists.txt` · `cpp/v8-bridge/include/machina_v8_bridge.h` · `cpp/v8-bridge/src/machina_v8_bridge.cpp` · `crates/native-core/src/lib.rs` · `crates/command-bus/src/lib.rs` · `crates/command-model/src/generated.rs`.
