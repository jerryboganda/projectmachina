---
title: "Personas and Jobs to Be Done"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define target users, their desired outcomes, constraints, and measures of value."
---

# Personas and Jobs to Be Done

## AI agent platform engineer

**Job:** When an agent must use a website, provide a fast, semantically rich, observable browser tool so the agent completes tasks with fewer tokens, retries, and brittle selectors.

**Needs:** MCP/native commands, semantic deltas, verified actions, workflows, secrets, approvals, deterministic replay, typed errors.

**Success:** lower task latency and token cost with a high verified-success rate.

## Extraction/crawling engineer

**Job:** When processing many dynamic pages, execute enough browser behavior to obtain accurate structured data without paying full Chromium cost for every page.

**Needs:** resource profiles, concurrency, proxies, cookies, schema extraction, backpressure, responsible crawling controls, explicit fallback.

**Success:** higher successful pages per core-hour and predictable per-page cost.

## Automation/test engineer

**Job:** Run existing automation clients against a faster runtime while knowing exactly which commands and browser behaviors are certified.

**Needs:** CDP/BiDi, Playwright/Puppeteer/Selenium compatibility, contexts, frames, interception, downloads, dialogs, emulation, traces.

**Success:** reduced runtime and infrastructure cost without silent incompatibilities.

## Platform/SRE administrator

**Job:** Operate multi-tenant browser workloads safely and economically.

**Needs:** fleet health, quotas, isolation tiers, autoscaling, telemetry, circuit breakers, incident tools, restore and rollback.

**Success:** SLO compliance, bounded cost, rapid recovery, low noisy-neighbor impact.

## Security/compliance administrator

**Job:** Permit browser automation without exposing internal networks, secrets, tenant data, or uncontrolled high-impact actions.

**Needs:** sandboxing, egress policy, audit, vault references, redaction, retention controls, approval gates, SBOM, incident response.

**Success:** demonstrable controls and no unresolved critical exposure.

## Workflow author/operations analyst

**Job:** Turn a successful browser interaction into a maintainable repeated process.

**Needs:** recorder, readable workflow DSL, test mode, variables, schedules, versions, run history, approvals, repair suggestions.

**Success:** repeatable outcomes without an LLM on normal runs.
