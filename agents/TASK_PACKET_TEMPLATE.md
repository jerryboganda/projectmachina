---
title: "Agent Task Packet Template"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide a portable, self-contained task contract for any coding agent."
---

# Agent Task Packet Template

```yaml
task_id: Mx-Tyy
title: <imperative title>
status: ready
priority: critical-path | high | normal | low
primary_role: <role-id>
required_reviewers: [reviewer]
dependencies: []
base_commit: <sha resolved at claim time>
write_scope: []
read_scope: ['**']
forbidden_scope: []
feature_flags: []
human_gates: []
resource_ceiling:
  wall_time_minutes: 90
  cpu: local-policy
  memory: local-policy
  network: allowlisted
repair_limit: 3
```

## Objective

One paragraph describing the user or system outcome.

## Context

Links to only the necessary PRD, architecture, ADR, protocol, security, and source locations.

## Deliverables

- Concrete files, modules, schemas, commands, or artifacts.

## Acceptance criteria

- Observable and binary where possible.
- Include behavior for error, cancellation, and unsupported capability paths.

## Fast gate

```bash
# exact smallest commands required before review
```

## Deferred heavy validation

List WPT, differential corpus, fuzz, load, soak, chaos, and certification suites that will run later.

## Prohibited shortcuts

List silent no-ops, mocked production behavior, unsafe defaults, or fallback misreporting that would invalidate completion.

## Evidence required

- Pull request and commit.
- Command outcomes.
- Acceptance mapping.
- Capability/contract documentation update.
- Risks and handoff.
