---
title: "Agent Role Catalog"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define specialist responsibilities and review boundaries for multi-agent development."
---

# Agent Role Catalog

## Roles

| Role ID | Primary responsibility | Must not self-approve |
| --- | --- | --- |
| `orchestrator` | Queue, claims, dependencies, merge, state, escalation | Project complete or public claims |
| `architect` | Boundaries, ADRs, contracts, cross-component invariants | Its own high-impact ADR without review |
| `native-engine` | Parser, DOM, V8, event loop, network, storage, scheduler | Unsafe/FFI security decisions |
| `protocol` | Command bus, HTTP, gRPC, CDP, BiDi, MCP, SDKs | Compatibility claims |
| `frontend` | SvelteKit console, design system, accessibility, client performance | Accessibility waiver |
| `platform` | Build, CI/CD, containers, Kubernetes, data services, observability | Production deploy |
| `security` | Threat model, sandbox, auth, egress, secrets, privacy, supply chain | Material risk acceptance |
| `performance` | Benchmarks, profiles, memory/CPU optimization, regression gates | Public benchmark claim |
| `quality` | WPT, differential, fuzz, conformance, reliability | Waiver of critical failures |
| `reviewer` | Independent diff and evidence review | Its own implementation |
| `release` | Versioning, artifacts, rollout, rollback, release evidence | GA approval |

## Assignment rules

- One primary role owns each task.
- Add mandatory reviewers according to risk: security-sensitive tasks require `security`; public protocol tasks require `protocol`; runtime/unsafe tasks require `native-engine`; deployment tasks require `platform`.
- A role may be fulfilled by a human, a subagent, or a separate coding product, but review independence must be preserved.

## Reviewer return format

```markdown
## Review outcome
- Verdict: approve | changes-required | blocked
- Blocking findings:
- Non-blocking findings:
- Acceptance criteria checked:
- Security/compatibility impact:
- Tests/evidence inspected:
- Suggested repair order:
```
