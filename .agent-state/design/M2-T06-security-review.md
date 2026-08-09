# Threat Model / Safety Review — M2-T06 (Narrow C++ V8 Bridge + Safe Rust Facade)

> Produced by a wave-1 security research agent ahead of M2-T06 implementation.
> Read-only review; no code changes. Feed this directly into the M2-T06 builder prompt.

Scope: `crates/runtime-v8` (currently empty scaffold — only `.gitkeep`), consumed by `crates/command-bus` (`CommandContext`, `CancellationToken`) and `crates/native-core` (`LifecycleEngine`, `EngineAdapter`), as defined in `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (search `## M2-T06`), constrained by `architecture/ADR/ADR-002-RUST_V8.md`, the "Rust and V8 boundary rules" in `research/TECHNOLOGY_SELECTION.md`, and `security/SECURITY_ARCHITECTURE.md` / `security/SECURITY_REVIEW_CHECKLIST.md`.

Trust boundary: every byte of script source, every JS value, every callback re-entry, and every timing/allocation pattern is adversarial (per `security/SECURITY_ARCHITECTURE.md` assumptions). The Rust facade is the trust boundary; V8 and the C++ shim are treated as a hostile-input-processing black box that must never be allowed to corrupt Rust-side memory, escape its thread, outlive its handles, or starve the process.

## Risk register (summary)

| # | Threat | Severity | Precondition |
|---|---|---|---|
| 1 | Use-after-free / double-free / type confusion across FFI | Critical | Any malformed script, GC timing, or missing ownership discipline in the shim |
| 2 | Cross-isolate/cross-thread handle use | Critical | Multi-isolate worker pool without thread-affinity checks |
| 3 | Stale handle after context/isolate disposal | Critical | Handle retained past `Dispose`/teardown without generation check |
| 4 | C++ exception or Rust panic unwinding across FFI | Critical (UB) | Any throwing STL call or Rust panic inside an `extern "C"` fn without a catch boundary |
| 5 | Unbounded heap/time/recursion from a single script | High | No isolate constraints/watchdog wired to `CommandContext.deadline` |
| 6 | Detached/neutered ArrayBuffer TOCTOU | High | Raw pointer into backing store held across a re-entrant JS call |
| 7 | Untrusted/unpinned V8 build (supply chain) | Critical | CI does not verify V8 revision/artifact provenance |
| 8 | Silent fail-open on internal error | High | Missing typed-error contract; default returns success/no-op |

## 1. Memory-safety boundary (Rust facade invariants)

- Opaque, `repr(C)` ABI only — no Rust struct mirrors a C++ class layout.
- Single owner per resource, RAII on the Rust side; dispose called exactly once from `Drop`.
- `Local` handles never persist across a call boundary — convert to `v8::Global<T>` inside a `HandleScope` before returning; never store a raw `v8::Local` in a struct field.
- Length-prefixed buffers, validated before use (prevents integer-overflow-driven under-allocation).
- Tag-checked casts (`IsString()`/`IsObject()`/etc.) before every native downcast of JS-originated data.
- ArrayBuffer/TypedArray backing-store TOCTOU: re-validate (`IsDetached()`, length) immediately before every use if a raw pointer is ever held across a call that can re-enter JS; prefer copying out synchronously instead.
- ABI/version self-check at init: shim-exported ABI version + V8 revision hash must match pinned values; mismatch is a hard init failure.
- No `unsafe` without a documented invariant + focused test (AGENTS.md).

## 2. Cross-thread and stale-handle misuse

- Thread-affinity token per isolate; every entry point checks caller token before touching V8 state. Recommended default: one isolate owned by exactly one OS thread/task for its whole lifetime.
- Generation counters `(isolate_id, generation)` on every handle wrapper; bumped on context disposal, isolate teardown, GC-epoch boundaries. Every call checks generation before dereferencing.
- Poison interior pointer on disposal in addition to bumping generation (defense in depth).
- Active-call depth counter per isolate; refuse disposal while calls are active; refuse new dispatch into a mid-disposal isolate.
- Required negative tests: cross-thread call → `CrossThreadAccess`; call-after-context-dispose → `StaleHandle`; drop-after-isolate-teardown (no UAF/double-free); repeated create/dispose cycles clean under ASan.

## 3. Exception/panic containment

- Every `extern "C"` fn callable from C++ wraps its body in `catch_unwind`; crate stays `panic = "unwind"` (not `abort`).
- A caught panic poisons/retires that isolate — no continued use of a wrapper whose invariants may be violated.
- Every C++ ABI entry point catches `(...)` or is built exception-free for V8-adjacent code.
- Every `Script::Run`/`Function::Call` wrapped in `v8::TryCatch`; JS exceptions become typed `ScriptException{message, stack, name}` values, never propagate as native exceptions.
- FFI signatures: `extern "C" fn(...) -> StatusCode` with out-params, never `Result<T,E>` directly.

## 4. Resource exhaustion, tied to M2-T01's cancellation/deadline model

- Isolate heap constraints set from the session's `ResourceBudget` (already in `machina_session`) at creation.
- Near-heap-limit callback: bounded bump + forced GC, then `TerminateExecution` with typed `MemoryBudgetExceeded` — never let V8 OOM-abort the process.
- Execute-script entry point takes a view of `CommandContext`; arms watchdog against `context.deadline`; calls `TerminateExecution()` from the owning scheduler/event-loop timer. After `Run` returns, distinguish via `TryCatch::HasTerminated()`/`IsExecutionTerminating()`: deadline → `DeadlineExceeded`; cancellation token fired → `CommandCancelled`; else → `ScriptException`. One deadline/cancellation source of truth only.
- Terminated isolates are retired, not reused, until reuse safety has specific evidence.
- Native (not just JS) call-stack depth bounded independently via a call-depth counter in host-function trampolines (protects against getter→native→script→getter recursion overflowing the native C stack, which isn't a catchable RangeError).
- Bounded shim-internal structures (persistent-handle table, pending microtask/callback queue) with explicit caps.

## 5. Build hardening

- Sanitizer build (ASan+UBSan) over C++ shim unit/lifecycle tests is the fast gate itself.
- Dedicated TSan job for thread-affinity tests specifically (data races are ASan's blind spot).
- Fuzz targets: (a) script-source fuzzer through compile+run via the safe facade; (b) lifecycle-sequence fuzzer (arbitrary orderings of create/dispose/call) hunting UAF and stale-handle-check bypasses.
- Debug/DCHECK-enabled V8 build in CI so V8's internal invariant assertions fire during test/fuzz runs.
- ABI/version compatibility as an automated CI gate (not just runtime assert): pin exact V8 revision, hash the generated C ABI header, fail CI on drift.
- Supply-chain provenance/checksum verification for the pinned V8 artifact — gates merge, not just release.

## 6. Pre-merge checklist (builder + reviewer)

**Memory safety:** opaque ABI only · single RAII owner per resource · no persisted `Local` handles · length-validated buffers · tag-checked downcasts · backing-store TOCTOU re-validated · ABI/version checked at init · every `unsafe` block documented+tested.

**Thread/handle safety:** isolate ownership policy documented · thread-affinity token checked on every entry · generation-checked handle wrappers · disposal poisons pointer · re-entrant disposal refused · cross-thread/stale-handle/repeated-lifecycle negative tests pass under ASan.

**Exception/panic containment:** every Rust `extern "C"` fn wrapped in `catch_unwind`, crate stays `panic=unwind` · caught panic retires the isolate · every C++ entry point exception-safe · every V8 call wrapped in `TryCatch` with typed `ScriptException` · no FFI fn returns `Result<T,E>` directly.

**Resource exhaustion / deadline integration:** heap constraints from `ResourceBudget` · near-heap-limit callback enforces typed error before OOM-abort · execute-script takes `CommandContext` and arms `TerminateExecution` against `context.deadline` (single source of truth) · `DeadlineExceeded`/`CommandCancelled`/`ScriptException` correctly distinguished · terminated isolates retired · native call-stack depth bounded independently.

**Build/CI hardening:** ASan+UBSan green · TSan (thread-affinity tests) green · fuzz harnesses (script-source + lifecycle-sequence) run, zero findings · V8 revision pin + ABI hash check enforced in CI · V8 artifact provenance/checksum verified · capability snapshot updated for newly reachable command surfaces · unresolved risks recorded in `agents/BLOCKERS.md`.

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (M2-T06, M2-T01) · `AGENTS.md` · `architecture/ADR/ADR-002-RUST_V8.md` · `research/TECHNOLOGY_SELECTION.md` · `security/SECURITY_ARCHITECTURE.md` · `security/SECURITY_REVIEW_CHECKLIST.md` · `quality/FAST_INNER_LOOP.md` · `crates/command-bus/src/lib.rs` · `crates/native-core/src/lib.rs` · `crates/runtime-v8` (confirmed empty) · `agents/CURRENT_STATE.md`.
