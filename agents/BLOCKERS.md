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

## BLK-003 — M1 real Chromium process launch is unimplemented (infra half resolved)

- Related task(s): M1-T12, M1 exit, M2 native corpus entry, M3-T15 (chromium-adapter launch code)
- First observed: 2026-08-09
- Updated: 2026-08-09 — local host has no Docker, but real Docker infra is now
  verified on owner-controlled shared VPS `185.252.233.186` (project
  `machina-m1-verify`, isolated 127.0.0.1-only ports, no public exposure).
  `postgres:16.4`, `redis:7.4.1`, `minio` and the fixture/observability
  containers came up healthy and answered real `pg_isready`/`redis-cli
  ping`/HTTP probes; `node --test scripts/test/m1-compatibility-smoke.test.mjs`
  passed on that host too. See `.agent-state/evidence/M1-T12-vps-runtime.md`.
- Owner: platform / owner approval
- Class: technical (downgraded from external — the remaining gap is code, not
  environment)
- Severity: high
- Exact reproduction/evidence: `rg -n "spawn|process::Command|CDP|remote-debugging" crates/chromium-adapter/src/lib.rs` returns no matches. `crates/chromium-adapter` and `services/worker-chromium/README.md` define the adapter/pool boundary and explicitly require an **injected** transport; no code path in the repository spawns a Chromium process, connects over CDP, or binds a real HTTP/gRPC listener. This holds true regardless of Docker/VPS availability.
- Work attempted: M1 source/client smoke runs the deterministic fixture journey through HTTP, gRPC, TypeScript, and Python command surfaces, verifies ordered reconnect events, typed cancellation/unsupported/worker-loss failures, and control/worker restart reconciliation — now demonstrated on both the local host (injected infra) and a real-Docker VPS host (real Postgres/Redis/MinIO, still injected command core).
- Why autonomous repair is exhausted: Provisioning Docker locally still requires host administrator approval (unresolved on this machine); that is now a non-blocking convenience issue since the VPS supplies real infra. The actual remaining work — implementing Chromium process launch and CDP wiring in `crates/chromium-adapter` — is an implementation task, not an environment blocker, and should be tracked as normal M3 work rather than kept open here.
- Impacted descendants: M1-T12 real-runtime acceptance and any production/container readiness claim; M2/M3 native corpus gates that assume a working Chromium launch path.
- Unaffected work that may continue: Native source implementation and all contract/unit tests that do not claim a real Chromium process. Docker-backed integration tests can now target the VPS if a durable arrangement is agreed.
- Recommended resolution: (1) Decide whether the VPS should be a durable CI/dev Docker target or was a one-off verification — if durable, document it in `delivery/DEVELOPMENT_ENVIRONMENT.md`. (2) Implement real Chromium launch/CDP connection in `crates/chromium-adapter` under a tracked M3 task. (3) Rerun the M1 real listener/worker journey against that implementation before declaring M1 exit.
- Human decision required, if any: Confirm the VPS is an acceptable durable Docker target for this purpose (shared production infra hosting other tenants), or provision a dedicated/local Docker host instead.
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
