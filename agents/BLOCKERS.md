---
title: "Blocker Register"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Track only proven blockers with evidence, impact, owner, and recommended resolution."
---
# Blocker Register

## Open blockers

## BLK-005 — EngineSession is `!Send`/`!Sync` once it composes machina-events

- Related task(s): interaction.click.v1 wiring (merged), any future async multi-worker protocol-http path
- First observed: 2026-08-09
- Owner: native-engine + protocol
- Class: technical
- Severity: low (no current caller requires `Send`/`Sync` on `EngineAdapter`)
- Exact reproduction/evidence: `machina_events::EventTargetRegistry` is `Rc`-based by design (per `.agent-state/design/M2-T11-event-dispatch-design.md`); `NativeEngine`/`ChromiumEngine` now compose it via `EngineSession`, making both `!Send`/`!Sync`. Verified by the wiring task's builder before merge — no test failure, purely a forward-compatibility note.
- Work attempted: none yet; explicitly deferred as out of scope for the click-wiring task.
- Why autonomous repair is exhausted: not attempted — no current consumer needs this, premature to redesign now.
- Impacted descendants: any future architecture putting `EngineAdapter` instances behind `Arc` for a multi-threaded/async worker pool (plausibly M2-T08's event loop or a later protocol-http concurrency change).
- Unaffected work that may continue: everything else; the current single-threaded-per-session model this repo already uses elsewhere (`crates/dom`, `crates/events` are both deliberately single-thread-confined) is unaffected.
- Recommended resolution: when a task actually needs `Send`/`Sync` `EngineAdapter` instances, either wrap the `Rc`-based state behind a lane-actor pattern (matching `.agent-state/design/M2-T08-event-loop-design.md`'s `V8LaneHandle`/`ExecutionLane` design, which already solves exactly this problem for `runtime-v8`) or reassess whether `machina-events` should offer a `Send`-friendly alternative storage mode.
- Human decision required, if any: None yet.
- Review date: Before any task puts `EngineAdapter` behind `Arc`.

## BLK-004 — M2-T02 network loader: security-review items deferred to M3-T06

- Related task(s): M2-T02 (merged), M3-T06
- First observed: 2026-08-09
- Owner: native-engine
- Class: technical
- Severity: low (explicitly scoped-deferred, not blocking anything today)
- Exact reproduction/evidence: `.agent-state/design/M2-T02-security-review.md`'s pre-merge checklist marks several items `[DEFER→M3-T06]`; M2-T02's merged evidence (`.agent-state/evidence/M2-T02.md`) confirms every `[NOW]` item was implemented and tested, and lists the deferred set explicitly rather than silently dropping it: full cookie-jar/CORS-credential-aware redirect forwarding, referrer-policy computation across redirects, HTTP caching semantics, HTTP/2 server push and HTTP/3, proxy support (CONNECT/SOCKS), a DNS-rebinding-hardened resolver/cache with tenancy-aware variants, and certificate pinning/strict OCSP policy. Additionally: connection pooling/concurrency caps were not implemented in this pass (every request is a fresh connection through the full policy pipeline) — a real pre-production follow-up, not a security gap per se.
- Work attempted: `crates/network`'s `[NOW]` scope (SSRF/DNS-rebinding defense, redirect re-validation, decompression-bomb/oversized-response/malformed-header defense, TLS hard-enforcement, per-phase timeouts/budgets, streaming backpressure) is implemented and covered by 52 passing tests including dedicated SSRF/malformed-response fixtures.
- Why autonomous repair is exhausted: not applicable — this is intentionally out of M2-T02's scope per the milestone doc's own task boundary (M3-T06 owns CORS/referrer/cache/redirect-credential hardening).
- Impacted descendants: M3-T06 inherits this list as its starting scope; M2-T09 (navigation, wires `machina-network` into `native-core`) should not assume cookie/CORS/cache semantics exist yet.
- Unaffected work that may continue: everything else in M2 — this does not block M2-T08/T09/T12/T14.
- Recommended resolution: M3-T06 implements the deferred items against this list; add connection pooling as a separate, tracked pre-production task once real traffic patterns are known.
- Human decision required, if any: None.
- Review date: Before M3-T06 is claimed; before any production traffic claim.

## BLK-003 — M1 real Chromium/process compatibility runtime unavailable

- Related task(s): M1-T12, M1 exit, M2 native corpus entry
- First observed: 2026-08-09
- Owner: platform / owner approval
- Class: external
- Severity: high
- Exact reproduction/evidence: `Get-Command docker` returns no Docker executable; the recorded Docker Desktop installation attempt failed at administrator/UAC. The Chromium adapter remains an injected boundary and no HTTP/gRPC listener or real Chromium process is available in this local session.
- Work attempted: M1 source/client smoke now runs the deterministic fixture journey through HTTP, gRPC, TypeScript, and Python command surfaces, verifies ordered reconnect events, typed cancellation/unsupported/worker-loss failures, and control/worker restart reconciliation.
- Why autonomous repair is exhausted: Real browser/container provisioning and privileged runtime installation require host administrator approval outside the repository session.
- Impacted descendants: M1-T12 real-runtime acceptance and any production/container readiness claim.
- Unaffected work that may continue: Native source implementation and all contract/unit tests that do not claim a real Chromium process.
- Recommended resolution: Install Docker/Chromium runtime with administrator approval, run the M1 real listener/worker journey and crash/disconnect matrix, then replace this blocker with verified evidence.
- Human decision required, if any: Owner approval for privileged runtime installation.
- Review date: Before beta/RC/GA and before M1 exit is declared.

## BLK-002 — Docker/Compose unavailable for M0-T11 health evidence

- Related task(s): M0-T11
- First observed: 2026-08-09
- Owner: platform
- Class: external
- Severity: medium
- Exact reproduction/evidence: `Get-Command docker` returns no executable in the current host; `winget install Docker.DockerDesktop` downloaded and verified the installer but failed at the administrator/UAC install step.
- Work attempted: Compose source and local-only lifecycle scripts were implemented; source checks and non-Docker tests pass; Docker Desktop provisioning was attempted twice.
- Why autonomous repair is exhausted: Docker Desktop installation/runtime requires an interactive administrator approval outside the repository session.
- Impacted descendants: M0-T11 exit and M0-T12 full local-stack rehearsal.
- Unaffected work that may continue: M0-T01 through M0-T10 source review and all tasks not requiring container health.
- Recommended resolution: Install Docker Desktop/Engine, run `just dev-up`, `just dev-health`, and `just dev-reset --confirm` against the local project only.
- Human decision required, if any: None.
- Review date: Before M0 exit.

## Closed blockers

## BLK-001 — Terminal validation and repository integration unavailable in current session

- Related task(s): M0-T01, M0-T02, M0-T03, M0-T04, M0-T06, M0-T09
- First observed: 2026-08-09
- Owner: local implementation session
- Class: technical
- Severity: high
- Exact reproduction/evidence: The initial folder-backed session exposed no terminal, Git command, child-session, worktree, or pull-request operation.
- Work attempted: Implemented the bootstrap, contract, claims, redaction, telemetry, policy, and CI source files directly in the local working tree.
- Why autonomous repair is exhausted: Resolved when the terminal-capable agent was enabled.
- Impacted descendants: Initial validation and Git integration.
- Unaffected work that may continue: All local source implementation.
- Recommended resolution: Closed by enabling terminal/Git tooling and running the available fast gate.
- Human decision required, if any: None.
- Review date: Before first commit.

## Blocker template

```markdown
## BLK-<number> — <title>

- Related task(s):
- First observed:
- Owner:
- Class: technical | external | human-gate | security | legal | data
- Severity: critical | high | medium | low
- Exact reproduction/evidence:
- Work attempted:
- Why autonomous repair is exhausted:
- Impacted descendants:
- Unaffected work that may continue:
- Recommended resolution:
- Human decision required, if any:
- Review date:
```

## Closure rule

A blocker closes only when its task is unblocked by verified evidence, formally descoped through product change control, or waived by the authorized owner. “Could not solve” is not sufficient without reproduction and bounded attempts.
