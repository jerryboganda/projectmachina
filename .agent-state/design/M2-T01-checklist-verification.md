# M2-T01 checklist spot-check (read-only verification)

> Produced by a wave-2 protocol agent, checking the merged M2-T01 (PR #28, commit f38692d3)
> against the wave-1 contract-compatibility checklist's "priority checklist for the M2-T01 builder."

## Verdict: 4 followed, 1 partially-applicable-not-exercised, 1 not-applicable. No evidence drift found.

| # | Rule | Verdict |
|---|---|---|
| 1 | No new `EngineAdapter` methods — use `CommandKind` arms instead | **Followed** — trait unchanged (3 methods); new ops are inherent methods on `NativeEngine`/`ChromiumEngine`/`LifecycleEngine`, not trait methods, and not new `CommandKind` arms (none were needed — see #2). |
| 2 | Explicitly decide bus-command vs. internal-state for context/page ops before coding | **Followed** — documented in module doc-comment + evidence: context/page ops are direct Rust methods since the schema has no `context.create.v1`/`page.create.v1` yet. `CommandKind` enum confirmed unchanged (still 5 variants). |
| 3 | Register new capability ids + "never claims readiness before subsystem ready" fast-gate test | **Partially applicable, not exercised** — no new capability id was needed (consistent with #2), but the "readiness" guard test itself still doesn't exist anywhere in the codebase. Whichever task first adds an async-initialized capability (flagged: likely M2-T07/V8 warm-up) has to build this guard from scratch, no precedent yet. |
| 4 | Don't regress `SessionCreateV1`/`SessionCloseV1` or the two existing tests | **Followed** — both tests present under original names, pass (8/8 native-core, 11/11 session), observable command behavior unchanged. |
| 5 | New crate dir → add to workspace members, native-side deps only, ideally fix check-boundaries.mjs | **Not applicable** — no new crate directory; only pre-existing `session`/`native-core` modified, `Cargo.toml` files byte-identical. (The boundary-checker fix is being done separately by a wave-2 tooling agent regardless.) |
| 6 | Don't touch `m1-compatibility-smoke.mjs` unless covered `CommandKind` behavior changes | **Followed** — file untouched (empty diff). |

## Evidence-file accuracy

`.agent-state/evidence/M2-T01.md` checked against the real diff: **no material drift**. Changed-files list, test names/counts, import-isolation claims, and the `check-boundaries.mjs` pass claim all independently reproduced correctly. Notably candid about what it did *not* do (no new capability id registered, explained why; full `fast-gate.yml` explicitly flagged as not run in-sandbox) — honest under-claiming, not inflation. Minor unverified item: the workspace-wide "17 test binaries / 16 crates" count is self-reported, not independently re-run here, but nothing contradicts it.

## Capability-registry / router composition — the substantive finding

M2-T01 changes **zero** lines in `crates/capability` or `crates/command-bus`. In the narrow sense, "native engine plugs into the M1 router without redesign" is trivially true — but only because the new context/page/resource/cancel/health surface was deliberately built **entirely outside** the bus/router path (direct Rust methods, never through `EngineAdapter::execute`/`CommandBus::decide`/`capability_registry()`).

**Consequence:** M1's exit-criteria promise is not actually re-tested by M2-T01 beyond what M1 already proved for session create/close. The Rust-level `EngineSession` API is genuinely clean and reusable (well-isolated, symmetric Native/Chromium, no protocol imports) — that part of the composition works. But the router/capability-registry leg of the same promise is **still open, not closed** — it gets its first real test only once some task adds a `context.create.v1`/`page.create.v1` `CommandKind` and routes it through `CommandBus::execute`. That's where any real friction (e.g. whether `CommandEnvelope.context_id`/`page_id` suffice for routing, or whether the `CapabilityStatusRecord.limitations` granularity gap the wave-1 checklist flagged becomes a live problem) will actually surface. Flag this explicitly to whoever picks up the first schema-backed context/page command.
