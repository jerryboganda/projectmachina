---
title: "Svelte 5 and SvelteKit Frontend Architecture"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define a lightweight, accessible, generated-contract web console and documentation experience."
---

# Svelte 5 and SvelteKit Frontend Architecture

## Decision

Use Svelte 5, SvelteKit, and TypeScript for the authenticated console and documentation/public surfaces. Use SvelteKit's static adapter/prerendering for documentation and marketing routes where possible; deploy the authenticated console with an appropriate server adapter. Use plain Svelte + Vite only for a genuinely embedded widget that does not need SvelteKit routing/data/server features.

## Why

- Compiled component model with limited runtime overhead.
- First-party application framework for routing, server/data loading, forms, prerendering, adapters, and deployment.
- Strong fit for real-time trace views and interactive workflow tooling without adopting a heavier framework solely for ecosystem convention.
- Shared TypeScript contracts can be generated from canonical schemas.

## Applications

### `apps/console`

Authenticated developer/operator/admin console.

Primary routes:

```text
/login
/orgs/[orgId]/projects
/projects/[projectId]/overview
/projects/[projectId]/sessions
/projects/[projectId]/sessions/[sessionId]
/projects/[projectId]/workflows
/projects/[projectId]/workflows/[workflowId]
/projects/[projectId]/usage
/projects/[projectId]/settings
/admin/fleet
/admin/incidents
```

### `apps/docs-site`

Prerendered documentation, SDK references, compatibility matrix, changelog, and examples. Dynamic API explorer is an isolated client-enhanced route.

## State model

- URL is the source of navigational/filter state where practical.
- Server `load` functions fetch initial authorized data.
- Generated client handles command API and event-stream reconnection.
- Local reactive state is scoped to a page/component; avoid a global store by default.
- Optimistic updates require idempotency key and rollback behavior.
- Session/trace event buffers are bounded and virtualized.

## API contract

Frontend imports generated TypeScript types and clients from `packages/contracts-ts`. It does not duplicate enums, error codes, capability IDs, or policy schemas. Contract generation is checked in CI.

## Design system

`packages/ui` provides accessible primitives, tokens, layout, forms, tables, dialogs, decision cards, timelines, code/log viewers, charts, and status badges. Components expose semantic HTML, keyboard behavior, focus management, and reduced-motion support.

## Performance budgets

- Public/docs routes prerender by default.
- Route-level code splitting.
- No large visualization dependency without bundle review.
- Virtualize high-volume event/log tables.
- Stream or paginate traces; never load an unbounded session artifact into memory.
- Define per-route JS and interaction budgets during M6 and enforce in final audit.

## Security

- HttpOnly secure session cookies or approved token design.
- Server-side authorization remains authoritative.
- Content Security Policy, CSRF defenses, secure headers, output encoding, and no unsafe HTML from page content.
- Display hostile page strings as text; screenshots/artifacts use isolated origins/downloads.
- Secrets are referenced, never returned to the browser after creation.

## Accessibility

Target WCAG 2.2 AA. Require keyboard completion of all critical workflows, clear focus, non-color status indicators, screen-reader labels, logical headings, accessible live updates, and manual assistive-technology checks during M9.

## Error experience

Display canonical error code, human explanation, retryability, engine/fallback context, correlation ID, and safe next action. Never expose secret values or raw internal stack traces.
