---
title: "Definition of Done"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define task completion beyond code generation and make every result reviewable, testable, and resumable."
---

# Definition of Done

A task is done only when:

- [ ] Deliverables and every acceptance criterion are satisfied.
- [ ] Code builds/type-checks and the required fast gate passes on final diff.
- [ ] Relevant focused tests and negative/security tests are included.
- [ ] Public contract/schema/capability/docs are updated together.
- [ ] No silent unsupported behavior, placeholder production path, or undisclosed approximation remains.
- [ ] Independent review has no blocking finding.
- [ ] Branch is integrated through merge queue without unresolved conflict.
- [ ] Commands/results and acceptance evidence are recorded.
- [ ] Deferred heavy validation is recorded with requirement/capability IDs.
- [ ] New risk, decision, limitation, feature flag, or dependency is documented.
- [ ] Current state/work queue/capability evidence are updated.
- [ ] Task/path claims are released and next ready task is identified.

A WIP commit, generated code without integration, passing unit tests with failing compilation, or open blocking review is not done.
