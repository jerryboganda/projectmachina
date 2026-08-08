---
title: "M6 — Svelte Console and Developer Experience"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Program and Architecture"
purpose: "Provide executable task packets and exit criteria for milestone M6."
---

# M6 — Svelte Console and Developer Experience

## Objective

Deliver a lightweight, accessible Svelte 5/SvelteKit console and documentation experience for projects, sessions, traces, semantic actions, workflows, approvals, usage and operations.

## Entry criteria

- Stable M1/M4 API schemas exist.
- Workflow contracts are available for advanced routes; admin routes may develop against stable contracts in parallel.

## Exit criteria

- All critical beta journeys are usable through the console.
- Frontend meets milestone accessibility/security/performance gates.
- Docs and quick starts are generated, tested and versioned.

## Scheduling notes

The orchestrator may run at most two implementation tasks concurrently. Parallel tasks must have non-overlapping write scopes and no unmerged shared-contract dependency. Every task follows `agents/AUTONOMOUS_LOOP.md` and records deferred heavy validation.

## Tasks

## M6-T01 — Create SvelteKit application shell and accessible design system

**Primary role:** frontend  
**Dependencies:** M1-T07, M4-T02, M0-T01  
**Risk:** high  
**May run in parallel:** constrained

### Deliverables

- Initialize Svelte 5/SvelteKit apps, generated API client integration and route layouts.
- Build accessible tokens/primitives for forms, tables, dialogs, status, code/log, timelines and decision cards.
- Set bundle, accessibility, testing and CSP/security baselines.

### Acceptance criteria

- Application shell renders authenticated/unauthenticated states with keyboard navigation.
- Components pass focused accessibility tests and do not duplicate API enums.
- Static docs routes prerender and console server build succeeds.

### Fast gate

- Run Svelte check/build, component tests and accessibility lint.
- Inspect baseline route bundle and CSP headers.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T02 — Implement authentication, organization, project and policy console

**Primary role:** frontend + security  
**Dependencies:** M6-T01, M1-T02, M4-T02  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Build login/session handling and organization/project navigation.
- Implement members/credentials metadata/project policy views and safe mutation forms.
- Add version conflict, authorization and audit feedback.

### Acceptance criteria

- Critical management flows work by keyboard and server authorization is authoritative.
- Secret credential value is shown only at approved creation moment if design permits, then not retrievable.
- Stale policy update handles ETag/version conflict.

### Fast gate

- Run route e2e and cross-project authorization smoke.
- Run CSRF/output-encoding/security header checks.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T03 — Implement session launch, list, filters and lifecycle controls

**Primary role:** frontend  
**Dependencies:** M6-T01, M4-T02, M4-T03  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Create policy-aware session launch form with fidelity/fallback/isolation/deadline/network controls.
- Build paginated/filterable session list and status indicators.
- Implement cancel/close and typed error/fallback summaries.

### Acceptance criteria

- User can launch and control native/Chromium/hybrid sessions.
- Form prevents invalid combinations using generated schema while server remains authoritative.
- Live state updates recover after event reconnect.

### Fast gate

- Run session route e2e against fixture stack.
- Run slow stream/reconnect and error rendering tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T04 — Build live session timeline, trace and reproduction explorer

**Primary role:** frontend + observability  
**Dependencies:** M6-T03, M1-T10, M4-T03  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Render bounded virtualized lifecycle/command/network/console/fallback timeline.
- Add correlation navigation, safe detail panes, filters and artifact/reproduction requests.
- Implement stream cursor/backpressure/resync UI.

### Acceptance criteria

- Large synthetic trace remains within client memory/performance budget.
- Sensitive fields are redacted and hostile strings render as text.
- Missing/expired artifact and resync state are understandable.

### Fast gate

- Run large-trace performance/e2e smoke.
- Run XSS/canary/redaction UI tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T05 — Build semantic inspector and verified action playground

**Primary role:** frontend  
**Dependencies:** M6-T04, M4-T10, M3-T13  
**Risk:** high  
**May run in parallel:** yes

### Deliverables

- Display semantic tree/delta, roles/names/states/revisions and locator details.
- Allow permitted click/fill/press/select/check with pre/postcondition result.
- Show engine/capability/fallback and visual-certainty limitation.

### Acceptance criteria

- User can inspect and act on fixture without raw DOM dump.
- Ambiguity/stale/interactability errors are clear and no unsafe retry occurs.
- Delta updates do not lose selection silently.

### Fast gate

- Run semantic/action route e2e.
- Run large-tree virtualization and stale-revision tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T06 — Build workflow editor, recorder, versions and run history

**Primary role:** frontend  
**Dependencies:** M6-T01, M5-T09  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Implement recording controls, schema-aware workflow editor and validation diagnostics.
- Build version diff/activate/rollback, schedule and run history/step timeline.
- Show deterministic versus recovery execution and cost.

### Acceptance criteria

- User can record, validate, save, run and roll back a fixture workflow.
- Editor cannot save schema-invalid or unauthorized workflow.
- Run state resumes/reconnects without duplicate actions.

### Fast gate

- Run workflow UI e2e and editor validation tests.
- Run restart/reconnect view tests.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T07 — Build secrets, policies and approval decision experience

**Primary role:** frontend + security  
**Dependencies:** M6-T02, M6-T06, M5-T07  
**Risk:** critical  
**May run in parallel:** yes

### Deliverables

- Create secret-reference metadata management without value retrieval.
- Build high-impact approval inbox/detail card and decision flow.
- Display destination, data classes, workflow version, expiry and audit context safely.

### Acceptance criteria

- Approver can approve once/deny/abort under authorization.
- Secret values never enter browser response, state or logs.
- Expired/stale decision cannot approve a different run/step.

### Fast gate

- Run approval race/authorization e2e.
- Run canary secret browser-state scan.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T08 — Build fleet, usage, quotas, alerts and incident administration

**Primary role:** frontend + platform  
**Dependencies:** M6-T01, M1-T04, M4-T02  
**Risk:** critical  
**May run in parallel:** constrained

### Deliverables

- Create worker pool/capacity/version/health/drain views using stable admin APIs.
- Build usage/cost/fallback/quota dashboards and safe quota mutations.
- Add incident/circuit-breaker/emergency-control views with elevated authorization.

### Acceptance criteria

- Keep the interface contract stable so M7-T09 can supply full live fleet integration without redesign.
- Admin views are hidden and denied without role.
- Destructive/emergency action uses explicit confirmation, reason and audit.
- High-cardinality data is paginated/aggregated.

### Fast gate

- Run admin authorization and operation smoke using mock/stable contract.
- Run large-data dashboard performance test.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T09 — Build prerendered documentation site, API explorer and quick starts

**Primary role:** frontend + developer-experience  
**Dependencies:** M6-T01, M4-T11, M4-T12  
**Risk:** medium  
**May run in parallel:** yes

### Deliverables

- Create SvelteKit static/prerendered docs site sourced from versioned Markdown/generated references.
- Add safe API explorer against user-selected project and SDK quick starts.
- Publish capability/client matrix, errors, examples and changelog.

### Acceptance criteria

- Static docs build has no broken internal links/examples.
- Explorer never embeds long-lived credentials in URL/storage.
- Quick starts are tested from clean environments.

### Fast gate

- Run prerender/link/code-example checks.
- Run explorer auth/CSP/security smoke.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.

## M6-T10 — Complete frontend accessibility, security and performance milestone gate

**Primary role:** frontend + quality + security  
**Dependencies:** M6-T01 through M6-T09  
**Risk:** critical  
**May run in parallel:** no

### Deliverables

- Run critical-route e2e, automated accessibility/security and bundle/load audits.
- Perform manual keyboard and initial screen-reader review.
- Fix milestone blockers and record M9 deferred full audit.

### Acceptance criteria

- All critical beta journeys complete without raw API use.
- No critical accessibility/security issue and route budgets meet M6 targets.
- Hostile page strings/artifacts cannot execute in console origin.

### Fast gate

- Run full M6 e2e/accessibility/security/performance smoke.
- Run production SvelteKit build and prerender.

### Completion evidence

The implementing agent must link the pull request, list changed files, record commands and outcomes, update the capability matrix when applicable, and add unresolved risks to `agents/BLOCKERS.md`.
