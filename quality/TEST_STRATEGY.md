---
title: "Risk-Tiered Test Strategy"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Balance the requested end-loaded heavy testing with the minimum continuous checks required for fast completion."
---

# Risk-Tiered Test Strategy

## Decision

Project Machina uses **thin continuous validation and one exhaustive final campaign**. It does not run every lengthy suite at each step. It also does not postpone all testing, because an uncompiled or incompatible foundation would make late testing far slower than small early checks.

## Test tiers

### Tier 0 — author feedback

Formatter, compiler/type system, linter, selected unit test, local schema check. Run during implementation as needed.

### Tier 1 — required task fast gate

Runs once on the final task diff before review/merge. Target 1–8 minutes. See `FAST_INNER_LOOP.md`.

### Tier 2 — scheduled integration smoke

Run on main at a controlled cadence or after a cluster of related tasks:

- selected end-to-end sessions;
- small WPT shard for touched subsystem;
- small native-vs-Chromium differential set;
- one certified client canary;
- short fuzz seed corpus;
- migration and isolation smoke.

This detects cross-task drift without running the full campaign.

### Tier 3 — milestone hardening window

At M7/M8 and before release candidate:

- broader protocol matrix;
- larger target corpus;
- performance profile/regression set;
- security isolation/egress suite;
- hours-scale fuzz/load/soak;
- deployment/rollback drills.

### Tier 4 — M9 final heavy certification

One coordinated release-candidate campaign on a frozen build. Full details are in `FINAL_HEAVY_TEST_CAMPAIGN.md`.

## Test categories

- unit/property/component;
- contract/schema/serialization;
- integration and end-to-end;
- WPT standards;
- native/Chromium differential;
- CDP/BiDi/MCP/client conformance;
- security, tenant, sandbox, egress, secrets;
- fuzz/sanitizer;
- performance/load/capacity;
- reliability/soak/chaos;
- frontend accessibility/performance/usability;
- installation, upgrade, rollback, backup/restore, DR.

## Selection

Use the changed dependency graph, capability IDs, requirement IDs, and risk tags to select Tier 1/2 tests. A task packet may require more but not less than universal and risk-specific gates.

## Deferred-risk ledger

Each task records:

```yaml
deferred_validation:
  - suite: wpt/dom
    reason: broad shard scheduled for M8/M9
    requirement_ids: [FR-NAT-002]
  - suite: 24h-soak
    reason: final campaign
    requirement_ids: [NFR-REL-001]
```

The release lead verifies all deferred entries are executed or explicitly dispositioned.

## Repair after final campaign

The final campaign may reveal defects. Repairs use focused reproduction and fast gate first, then rerun only affected final shards plus mandatory regression, before final aggregate sign-off. Do not rerun every multi-day test for a documentation-only correction; use a documented impact decision.
