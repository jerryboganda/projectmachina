---
title: "Definition of Ready"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Ensure an autonomous agent can start a task without hidden dependencies or broad clarification."
---

# Definition of Ready

A task is ready only when:

- [ ] Stable task ID, title, objective, primary role, and milestone exist.
- [ ] Hard dependencies are complete and merged.
- [ ] Acceptance criteria are observable and not merely “implement feature.”
- [ ] Relevant requirement/capability/ADR/security references are available.
- [ ] Write scope can be claimed without overlap.
- [ ] Shared contract changes are merged first or one owner is designated.
- [ ] Fast-gate commands or selection rule are known.
- [ ] Deferred heavy test categories are identified.
- [ ] Required tools/environment are available.
- [ ] Human gates are either resolved or explicitly isolated from current work.
- [ ] Rollback/feature-flag expectation is known for risky behavior.
- [ ] No unresolved architecture conflict makes implementation unsafe.

The orchestrator may refine implementation details without asking the owner. It may not invent product semantics missing from accepted requirements; record a focused blocker or ADR question while continuing unaffected work.
