---
title: "Fast Inner Development Loop"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide the smallest mandatory validation that keeps autonomous coding fast without accumulating basic integration failures."
---

# Fast Inner Development Loop

## Objective

Keep ordinary task validation within approximately 1–8 minutes on a prepared developer machine while catching syntax, compilation, schema, ownership, and obvious behavioral defects before merge.

## Universal gate

Every code task runs:

1. repository format check for changed languages;
2. compile/type-check of changed packages and direct dependents;
3. changed or directly related unit/component tests;
4. schema/contract generation or compatibility check when contracts changed;
5. one focused smoke path for behavior changes;
6. secret/license/path-ownership checks;
7. documentation/capability update check where public behavior changed.

Documentation-only tasks run link/frontmatter/terminology checks and any generator validation they affect.

## Risk additions

| Change | Immediate focused checks |
| --- | --- |
| Parser/DOM/selector | targeted corpus + sanitizer/fuzz seed regression |
| V8/FFI/unsafe | sanitizer build/test for affected boundary + lifetime negative test |
| Auth/authorization | positive and cross-tenant negative tests |
| Network/proxy | SSRF/rebinding/redirect fixture relevant to change |
| Secrets/redaction | seeded canary secret scan |
| Sandbox/deployment | policy/config validation and negative launch check |
| Protocol/schema | round trip + adapter contract + compatibility descriptor check |
| State migration | checkpoint/replay side-effect test |
| Workflow action | pre/postcondition and approval/replay safety test |
| Frontend | type/check, affected component test, one route/e2e smoke, accessibility lint |

## Example command surface

The repository bootstrap should expose stable commands such as:

```bash
just fmt-check
just check-changed BASE_SHA=<sha>
just test-changed BASE_SHA=<sha>
just contract-check
just smoke TASK=<task-id>
just security-fast AREA=<area>
just docs-check
```

These commands select work from the diff and task metadata. Underlying compiler/test output remains visible and archived on failure.

## What is deliberately deferred

Do not run the full WPT suite, entire real-site corpus, all client versions, multi-hour fuzz, large load, 24/72-hour soak, chaos, penetration test, complete accessibility audit, or disaster-recovery rehearsal for each task.

## Merge gate

A task merges when:

- universal/risk-specific fast checks pass on final diff;
- acceptance criteria have focused evidence;
- independent review has no blocking finding;
- deferred heavy tests are recorded;
- main is healthy.

## Time-budget fallback

When a fast gate exceeds its budget:

1. profile the gate and split unrelated work;
2. cache toolchains/build outputs safely;
3. select tests by dependency graph;
4. move truly broad validation to scheduled/final suites;
5. never omit compilation, contract compatibility, or security-negative checks merely to hit time.
