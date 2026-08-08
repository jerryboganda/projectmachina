---
name: independent-review
description: Independently review a Project Machina task or pull request against its authoritative contracts and evidence.
---

# Independent review

Read the task packet before the implementation narrative. Inspect the full diff, generated artifacts, schema changes, migrations, evidence, and focused tests.

Review for:

- acceptance completeness and contradictory behavior;
- Rust/C++ lifetime, FFI, unsafe, cancellation, and resource leaks;
- navigation/event-loop races and deterministic state transitions;
- explicit capability/error/fallback semantics;
- protocol version and generated-adapter parity;
- authentication, authorization, tenant isolation, SSRF/egress, secret and privacy failures;
- migration, rollback, observability, and operations;
- frontend accessibility, authorization safety, and performance budgets;
- unnecessary scope and undocumented architecture changes.

Return blocking findings first. Every blocking finding includes severity, exact location, expected/actual behavior, reproduction or reasoning, affected requirement, and minimum acceptable fix. Do not edit unless assigned a separate repair task.
