---
title: "User Stories and Acceptance Themes"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Translate the product into outcome-focused stories that map to requirements and tasks."
---

# User Stories and Acceptance Themes

## Sessions and execution

- As a developer, I can create a session with explicit fidelity, isolation, network, locale, and deadline policies so execution is predictable.
- As a developer, I receive engine and fallback metadata for every task so I can understand cost and compatibility.
- As an operator, I can cancel a session and its network/script work promptly so runaway pages do not consume resources.
- As a tenant admin, I can set per-project quotas so one workload cannot exhaust the organization.

## Native engine

- As an extraction engineer, I can navigate JavaScript-heavy pages and query the resulting live DOM.
- As an agent developer, I can locate elements by semantic role/name/state and perform verified actions.
- As a test engineer, I receive explicit unsupported-capability errors rather than silent no-ops.
- As a developer, I can choose task-ready waits such as selector, semantic-stable, or function instead of waiting for all page resources.

## Fallback

- As a developer, I can allow, forbid, or require Chromium fallback.
- As a developer, I can see why fallback occurred and whether state was transferred or actions replayed.
- As an operator, I can measure fallback by domain, API, workload, and runtime version.
- As a security admin, I can require a stronger isolation tier when fallback launches a full browser.

## Protocols

- As a Playwright/Puppeteer/Selenium user, I can consult a certified version and capability matrix.
- As an MCP client, I can invoke stable typed browser tools and stream progress/events.
- As an SDK user, I receive consistent errors, cancellation, retries, and telemetry across languages.

## Workflows

- As an agent developer, I can record a successful session into a deterministic workflow.
- As an operator, I can version, test, approve, schedule, and roll back workflows.
- As a security admin, secrets appear as vault references and are never written to recordings.
- As a workflow author, normal runs execute without LLM cost, while optional recovery is explicit and bounded.

## Console and operations

- As a developer, I can inspect a session timeline, network metadata, console events, actions, semantic revisions, and fallback decisions.
- As an operator, I can download a redacted reproduction bundle.
- As an admin, I can inspect worker health, capacity, usage, errors, and incidents.
- As an owner, I can approve high-impact workflow actions from a clear decision card.

## Quality and trust

- As a customer, I can reproduce benchmark methodology.
- As a test engineer, I can see exactly which protocol modules and versions are certified.
- As a security reviewer, I can trace a capability to threat controls and tests.
- As an operator, I can restore durable state and roll back a release using rehearsed procedures.
