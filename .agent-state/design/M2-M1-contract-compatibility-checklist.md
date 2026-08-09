# M2 "Must Not Break M1" Compatibility Checklist

> Produced by a wave-1 protocol research agent. Read-only analysis; no code changes.
> CRITICAL — relay directly to the M2-T01 builder and every subsequent M2 builder.

## Key findings

1. **Do not add methods to `EngineAdapter`.** `CommandEnvelope` already carries `context_id`/`page_id: Option<String>`. Implement context/page identity, lifecycle, cancellation, and resource accounting via new `CommandKind` match arms inside `LifecycleEngine::execute`, dispatched through the existing single `execute()` entrypoint — not via a trait signature change. A trait change breaks every `impl EngineAdapter` (`NativeEngine`, `ChromiumEngine`, `ChromiumAdapter<T>`, test doubles in `protocol-http`/`chromium-adapter`).
2. **No `CommandKind` exists yet for context/page create-close or fetch/cookie/storage.** Decide explicitly and early whether these are bus commands (schema-additive, full regen chain: `schemas/command-model/v0.1/command-model.json` → `scripts/contracts/generate.mjs` → `crates/command-model/src/generated.rs` AND `packages/contracts-ts/src/command-model.ts` → `scripts/contracts/check.mjs`/`roundtrip.mjs`/`typecheck.mjs` → SCHEMA_VERSION/SHA bump → confirm M1-T11 SDKs still round-trip) or purely internal `NativeEngine`/`LifecycleEngine` state (no schema touch). This decision gates whether M2-T01/M2-T12 touch the pinned contract surface at all.
3. **`CommandKind` placeholders M2 must implement** (currently fall to native-core's `_ => Err(DispatchError::unsupported(...))` catch-all): `navigation.goto.v1` → M2-T09 · `dom.semantic_query.v1` → M2-T10 + M2-T13 · `interaction.click.v1` → M2-T11. `session.create.v1`/`session.close.v1` already work — M2-T01 must not regress them while extending for context/page.
4. **M2-T11 specifically:** `scripts/test/m1-compatibility-smoke.mjs` (line ~291-302) currently *asserts* `interaction.click.v1` returns `UNSUPPORTED_CAPABILITY` — that's a locked M1 baseline. M2-T11 making it succeed is an intentional behavior change and MUST update that smoke fixture explicitly in its PR, with a clear completion-evidence note that this is a reviewed, deliberate change — not an accidental regression fix.
5. **`CanonicalErrorCode` already has everything M2 needs** (`NAVIGATION_FAILED`, `SELECTOR_INVALID`, `ELEMENT_NOT_FOUND`, `ELEMENT_AMBIGUOUS`, `ELEMENT_NOT_INTERACTABLE`, `ACTION_POSTCONDITION_FAILED`, etc.) — M2 tasks should map into these existing codes, not invent new ones (new codes cost the same schema-regen/SDK-republish cycle as new `CommandKind`s).
6. **No documented per-command result-payload convention exists.** `CommandOutcome.result` is an opaque schema-pinned `String`. If M2-T09/T10/T13/T11 each invent ad-hoc JSON shapes independently, reconciling them later needs a breaking schema change. **Recommend M2-T01 (or a fast follow-up) define a minimal result-envelope convention** (e.g. `{"schema": "navigation.goto.result.v1", "data": {...}}`) before M2-T09/T13 land.

## Capability snapshot registration

- Register every new capability id in `CapabilitySnapshot` at construction time (`LifecycleEngine::new`), matching whatever string `required_capabilities` on the `CommandEnvelope` will carry — capability ids today are 1:1 with `CommandKind` wire names (`"navigation.goto.v1"`, `"dom.semantic_query.v1"`, `"interaction.click.v1"`).
- **Add a fast-gate test: the native adapter must never report a capability as `Native` before its backing subsystem is actually ready** — `capabilities()` returns an immutable reference fixed at construction, so a capability that only becomes available after async subsystem init (e.g. V8 isolate warm-up in M2-T07) cannot be registered lazily with the current API. Getting this wrong means `CommandBus::decide` routes work to an adapter that will fail.
- **Granularity gap to resolve before M2-T09/T12 land partial support** (e.g. HTTP/1.1 native but not HTTP/2): either (a) register finer-grained capability ids the router can check via `required_capabilities`, or (b) start populating the *already-schema-pinned but currently unused* `CapabilityStatusRecord.limitations: Vec<String>` for informational purposes. Recommend (b) — that struct already exists in the pinned schema/SDKs; avoid inventing a third parallel capability-status shape (there are currently two non-interoperating ones: `machina_capability::CapabilityEntry` (Rust-only, thin) vs. schema-pinned `CapabilityStatusRecord` (richer, unused by any code today)).
- Note: `CommandBus::capability_registry()` is rebuilt fresh from adapter snapshots on every call (not cached), and there is **no production endpoint exposing it yet** (`protocol-http` only routes `POST /v1/commands`, no `GET /v1/capabilities`; `crates/control-plane` has zero references to `capability`). The "M1-T09 router/capability-registry" surface is thinner than the milestone doc's phrasing implies — good to know when M2-T14's corpus gate needs to report capability status.

## Crate-boundary directionality — enforcement gap found (fix early)

Verified: `native-core`'s `Cargo.toml` currently has zero protocol-crate dependencies (compliant). `protocol-http` depends on `machina-native-core`/`machina-chromium-adapter` under `[dependencies]` (not `[dev-dependencies]`) for test-only usage — low risk today but should be tightened before M2 adds heavier native crates that would otherwise transitively bloat every protocol-crate build.

**`architecture/boundary-policy.json` + `scripts/architecture/check-boundaries.mjs` cannot actually catch a native→protocol violation today:**
1. No rule scans `crates/native-core` (or the future `dom`/`html`/`event-loop`/`network`/`navigation`/`runtime-v8`/`storage`/`semantic`/`extraction`) for forbidden references to protocol crates — only the reverse direction (protocol→native) is policed.
2. The checker never scans `Cargo.toml` dependency tables at all — only greps literal substrings in `.rs .ts .tsx .js .mjs .svelte` source files, so the actual Cargo dependency-graph edge is invisible to it.
3. **The forbidden-pattern strings use hyphens** (`"native-core"`, `"runtime-v8"`) **while real Rust import paths use underscores** (`machina_native_core::...`). A real violation like `use machina_native_core::NativeEngine;` inside a protocol crate would NOT be caught today — the check is functionally broken for its stated Rust-boundary purpose.

**Recommendation for whichever M2 task first adds crate content** (likely M2-T01/T02/T05/T06): fix `check-boundaries.mjs` to also scan `Cargo.toml` `[dependencies]`/`[build-dependencies]` against real package names, add both hyphen and underscore forms to `forbidden_patterns`, and add a reverse-direction rule with `roots` covering every native-side crate directory, forbidding `protocol-http`/`protocol-cdp`/`protocol-bidi`/`protocol-mcp`/`control-plane`/`scheduler`/`worker-pool`/`auth`/`policy`. Cheaper to do this once, early, than after 10+ unmonitored M2 crates exist.

## Workspace hygiene note (confirms the pattern security reviews also found)

Stub crates present but **not yet in workspace `[members]`**: `dom`, `html`, `event-loop`, `network`, `navigation`, `runtime-v8`, `storage`, `semantic`, `extraction`, `security-policy`, `state-bridge`, `protocol-cdp`, `protocol-bidi`, `protocol-mcp`. Whichever task first wires one of these in sets the dependency-edge precedent for the rest of M2 — keep `[dependencies]` limited to `command-model`/`capability`/`command-bus`/`session`/`telemetry`/other native-* crates only.

## Priority checklist for the M2-T01 builder specifically

1. Do not add methods to `EngineAdapter` — use new `CommandKind` match arms via the existing `execute()` entrypoint.
2. Decide explicitly whether context/page ops are bus commands (full regen chain) or purely internal engine state (no schema touch) — before writing code.
3. Register every new capability id at construction time; add the "never claims readiness before subsystem is ready" fast-gate test.
4. Do not regress the existing `SessionCreateV1`/`SessionCloseV1` arms or the two existing native-core tests (`shared_bus_executes_session_lifecycle_with_native_metadata`, `unsupported_navigation_is_explicit`); keep the `_ =>` unsupported catch-all for kinds this task doesn't implement.
5. If M2-T01 is also first to populate a new crate directory, add it to workspace `[members]`, keep dependencies native-side only, and ideally land the `check-boundaries.mjs` fix from above.
6. Don't touch `scripts/test/m1-compatibility-smoke.mjs` unless behavior for an already-covered `CommandKind` genuinely changes — if the PR diff touches it, that's a signal scope crept into a later task's territory (specifically M2-T11's).

## Files reviewed

`crates/command-bus/src/lib.rs` · `crates/capability/src/lib.rs` · `crates/command-model/src/lib.rs` + `generated.rs` · `crates/protocol-http/src/lib.rs` · `crates/protocol-events/src/lib.rs` · `crates/native-core/src/lib.rs` · `crates/chromium-adapter/src/lib.rs` · `crates/session/src/lib.rs` · `schemas/command-model/v0.1/command-model.json` · `scripts/contracts/generate.mjs` · `scripts/test/m1-compatibility-smoke.mjs` · `architecture/boundary-policy.json` · `scripts/architecture/check-boundaries.mjs` · root `Cargo.toml` · `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md`.
