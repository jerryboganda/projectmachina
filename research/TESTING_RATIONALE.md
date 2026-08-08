---
title: "Testing Cadence Rationale for Fast Agentic Delivery"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Quality, Performance, Security, and Program"
purpose: "Explain the risk-tiered compromise between rapid implementation and one exhaustive end-of-program certification campaign."
---

# Testing Cadence Rationale for Fast Agentic Delivery

## Owner objective

The owner wants the bulk of lengthy testing to run once near the end so coding agents spend less time repeatedly executing broad suites. The documentation honors that objective while retaining the smallest feedback needed to prevent weeks of work from accumulating on code that does not compile, schemas that disagree, or security boundaries that are already broken.

The strategy is therefore:

> **Tiny deterministic gates continuously; broad and expensive evidence in scheduled batches; one frozen exhaustive final campaign.**

It is not “no testing until the end.” Eliminating all early verification would usually slow a browser-engine program because root-cause distance and merge-conflict repair become much larger.

## Why a zero-test inner loop is unsafe here

Project Machina combines Rust, C++, V8, generated contracts, asynchronous networking, browser lifecycle behavior, protocol adapters, a web console, databases, worker isolation, and cloud deployment. A small interface error can fan out across many agents. The following failures must be caught close to the change:

- changed package no longer compiles or type-checks;
- generated protocol artifacts do not match their source schema;
- an unsafe/FFI invariant is violated;
- a database migration cannot apply or roll back in a disposable database;
- an authentication, secret, sandbox, or egress check is bypassed;
- a single critical smoke task cannot start, navigate, act, and close;
- a deterministic workflow can no longer replay its own fixture;
- a public command silently changes its error or event shape.

These checks are focused and usually bounded to minutes. They prevent the final campaign from becoming the first time subsystems are assembled.

## Test tiers

| Tier | Trigger | Typical time budget | Scope | Failure consequence |
| --- | --- | ---: | --- | --- |
| T0 static | Every changed task | 1–3 min | formatting, lint, schema validation, forbidden patterns, docs links for touched area | Task cannot complete |
| T1 changed-package | Every changed task | 2–8 min | compile/type-check and changed unit/contract tests | Task cannot complete |
| T2 focused behavior | Behavior change | 3–10 min | one or a few deterministic smoke paths for the changed capability | Task cannot merge |
| T2S security | Security-sensitive change | 5–20 min | focused abuse/negative tests for auth, egress, sandbox, secrets, parser, unsafe code | Security review required |
| T3 rotating integration | Scheduled and milestone boundary | Bounded batch | selected cross-component paths, protocol clients, migrations, native/fallback | Workstream may pause, not whole program |
| T4 broad hardening | M8 | Hours to days in CI | larger WPT slices, differential corpus, fuzz batches, load and failure injection | Release candidate cannot freeze |
| T5 final certification | M9 frozen candidate | One consolidated campaign | full selected WPT, protocol matrix, corpus, performance, fuzz, load, 24/72-hour soak, chaos, security, accessibility, backup/restore, rollback, DR | No GA until pass or formal waiver |

## What runs on each task

The task packet specifies an exact fast gate. It must include only checks that answer:

1. Does the changed code parse, compile, and satisfy local type/contracts?
2. Does the newly implemented behavior work on a deterministic fixture?
3. Did the change violate a high-risk boundary?
4. Is the change documented and resumable?

Agents must not opportunistically run the entire repository suite after every small change. CI uses path filters, dependency graphs, cached toolchains, generated-artifact checks, and test selection.

## What is intentionally deferred

Unless a task explicitly changes the harness itself, defer these to M8/M9 or scheduled batches:

- full Web Platform Tests;
- the entire real-site differential corpus;
- every certified Playwright/Puppeteer/Selenium version;
- long-duration fuzzing;
- maximum-concurrency load tests;
- 24-hour and 72-hour soak;
- full chaos/failover matrix;
- full penetration assessment;
- comprehensive accessibility pass;
- disaster recovery and regional-loss exercise;
- public benchmark reruns on all hardware classes.

## Final campaign structure

M9 begins only after feature freeze, dependency lock, schema freeze, clean migration rehearsal, and resolved critical blockers. The release candidate is content-addressed so every suite tests the same bits.

Recommended order:

1. reproducible build, SBOM, signatures, and provenance;
2. static/security scanning and license policy;
3. unit/contract/integration aggregate;
4. selected WPT and DOM/Web API conformance;
5. CDP, BiDi, MCP, HTTP/gRPC, SDK compatibility matrix;
6. real-site native/fallback differential corpus;
7. workflow record/replay and recovery corpus;
8. fuzz campaign and sanitizer builds;
9. load, saturation, fairness, and cost campaign;
10. crash, cancellation, dependency-failure, and chaos tests;
11. 24-hour then 72-hour soak after shorter gates pass;
12. security, privacy, abuse, and tenant isolation assessment;
13. Svelte console accessibility and performance budgets;
14. backup/restore, rollback, and disaster recovery drills;
15. benchmark publication reproduction;
16. release evidence review and accountable approval.

A failure creates a bounded correction branch. The corrected release candidate reruns the failed suite and every suite whose evidence could have been invalidated. The campaign does not blindly rerun unrelated expensive tests.

## Time-saving mechanisms

- content-addressed build and test artifacts;
- incremental Rust/C++/TypeScript caches;
- deterministic local fixtures and recorded network responses;
- path- and dependency-aware test selection;
- parallel CI shards for broad suites;
- fail-fast within a shard but continue independent shards for diagnostics;
- test-result reuse only when source, dependencies, toolchain, config, and fixture hashes match;
- quarantine only with owner, expiry, linked defect, and no impact on release claims;
- automatic minimal reproduction and reproduction bundle on failure;
- statistical benchmark comparison instead of ad-hoc timing;
- scheduled nightly/weekly rotation rather than full suite on every pull request.

## Quality accounting

Each task records deferred tests. The release evidence system aggregates them so “deferred” never means forgotten. A capability cannot be advertised merely because implementation merged; its matrix state remains `implemented-unverified` until the required conformance and final evidence pass.

## Waivers

A final-test failure may be waived only when:

- the affected capability is disabled or removed from public claims;
- security and data integrity are not weakened;
- impact and workaround are documented;
- an accountable owner signs the waiver;
- expiry and follow-up task exist;
- the release notes disclose user-visible limitations.

No waiver is allowed for a known tenant escape, secret exposure, arbitrary host access, corrupted durable state, unverifiable release artifact, or false public performance claim.

## Source basis

Web Platform Tests defines a cross-browser suite intended to give implementations confidence in compatibility. Its guidance favors short, cross-platform, self-contained tests and automated execution where possible. Project Machina uses WPT as a major conformance source but schedules broad runs into dedicated windows to preserve the fast coding loop.

Official references:

- WPT documentation: https://web-platform-tests.org/
- Running WPT: https://web-platform-tests.org/running-tests/index.html
- WPT test-suite design: https://web-platform-tests.org/test-suite-design.html
- WPT manual-test guidance: https://web-platform-tests.org/writing-tests/manual.html
