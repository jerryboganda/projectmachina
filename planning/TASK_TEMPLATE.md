---
title: "Planning Task Template"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Give planners and agents a standard structure for adding or splitting implementation work."
---

# Planning Task Template

```markdown
## Mx-Tyy — Imperative title

**Primary role:** role-id  
**Dependencies:** task IDs or `none`  
**Risk:** low | medium | high | critical  
**May run in parallel:** yes | constrained | no

### Objective
One observable outcome.

### Owned paths
- `path/**`

### References
- Requirement/capability/ADR/security links.

### Deliverables
- Concrete implementation and documentation artifacts.

### Acceptance criteria
- Binary observable outcomes, including error/cancellation/policy behavior.

### Fast gate
- Exact commands or selected suite.

### Deferred heavy validation
- WPT/differential/conformance/fuzz/load/soak/security/DR categories.

### Completion evidence
- PR, commit, commands/results, capability/requirement mapping, risks, handoff.
```

Keep tasks mergeable in roughly one bounded agent session when practical. Split by coherent contract or vertical behavior, not arbitrary file count.
