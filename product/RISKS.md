---
title: "Product and Program Risk Register"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Identify major technical, delivery, legal, security, and market risks with concrete mitigations."
---

# Product and Program Risk Register

| ID | Risk | Likelihood | Impact | Leading indicator | Mitigation |
| --- | --- | --- | --- | --- | --- |
| R-001 | Web API scope expands without bound | High | Critical | Native backlog grows faster than fallback reduction | Capability tiers, target corpus, fallback, scope admission test |
| R-002 | Native semantics diverge from Chromium | High | High | Differential failure clusters | WPT, differential harness, explicit capability status |
| R-003 | Protocol clients break on version changes | Medium | High | CI failures after client release | Pinned schemas, generated adapters, certified matrix |
| R-004 | Single-process speed harms isolation | Medium | Critical | Cross-session fault/data observations | Three isolation tiers, process boundaries, sandbox tests |
| R-005 | State migration is incomplete | High | High | Fallback task loses auth/storage/action state | Transfer contract, replay log, migration verification, explicit limitation |
| R-006 | Performance claims lack fairness | Medium | High | Success/fidelity differs across competitors | Reproducible benchmark and independent approval |
| R-007 | End-loaded testing reveals systemic incompatibility | High | Critical | Growing deferred failures and interface churn | Tiny fast gates, contract tests, scheduled smoke, final heavy campaign |
| R-008 | Agentic coding creates inconsistent architecture | Medium | High | Duplicate abstractions, overlapping edits, undocumented decisions | One command model, ADRs, file claims, independent review |
| R-009 | Agent sessions stop before completion | Certain | Medium | Stale claims, missing context | Checkpointed loop, durable state, leases, handoffs |
| R-010 | Hostile page escapes worker or reaches internal network | Medium | Critical | Sandbox/egress test failure | Defense in depth, deny defaults, dedicated/hardened tiers, audits |
| R-011 | Secrets leak through traces/recordings | Medium | Critical | Secret scanners/redaction failures | Opaque references, centralized redaction, restricted artifacts |
| R-012 | Licensing contaminates clean-room core | Low/Medium | Critical | copied code or unresolved dependency | Contribution policy, SBOM, provenance, legal review |
| R-013 | Chromium fallback dominates cost | Medium | High | Persistent fallback > target | Telemetry-ranked native APIs, domain profiles, better prediction |
| R-014 | V8/FFI memory safety defect | Medium | Critical | sanitizer/fuzz crash | Narrow bridge, ownership invariants, sanitizers, fuzzing |
| R-015 | Managed service abused for scraping or attacks | High | High | complaints, blocked traffic, abnormal origins | responsible-use policy, quotas, rate limits, identity, emergency block |
| R-016 | Frontend becomes heavy or distracts from runtime | Medium | Medium | bundle growth, API churn | SvelteKit, performance budgets, stable generated client, milestone separation |
| R-017 | Cloud costs exceed value | Medium | High | cost/task trend, idle pools | metering, autoscaling, warm-pool budgets, tenant limits |
| R-018 | Two agents create merge/conflict overhead | Medium | Medium | claim collisions and rework | contract-first tasks, worktrees, two-lane limit, telemetry |

## Risk review cadence

- Review active critical/high risks at each milestone exit.
- Any security incident or major benchmark/compatibility miss triggers immediate review.
- Close a risk only with evidence or convert it into an accepted residual risk with owner and expiry.
