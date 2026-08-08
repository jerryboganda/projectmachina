---
title: "Local Development Workflow"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide a fast daily workflow for engine, platform, protocol, and SvelteKit development."
---

# Local Development Workflow

## Initial setup

```bash
./scripts/dev/bootstrap
just doctor
just dev-up
just seed-local
```

The console should be reachable only on a local development address by default. Test credentials are synthetic and clearly marked.

## Common modes

```bash
just dev-native          # native worker with reload/restart helper
just dev-platform        # control plane and data services
just dev-console         # SvelteKit dev server + generated client
just dev-full            # complete local stack
just fixture-server      # deterministic multi-origin test sites
```

## Task workflow

1. Claim task and create worktree.
2. Run `just doctor` or area-specific check.
3. Implement against local fixtures.
4. Run `just fast-gate TASK=Mx-Tyy BASE_SHA=...`.
5. Inspect diff and evidence.
6. Commit, open PR, obtain review, merge, update state.

## Debugging

- Enable scoped structured logs by component/code, not blanket page-content logging.
- Use trace/reproduction bundle with local fixtures.
- Attach debugger/sanitizer to isolated native worker.
- Use Chromium devtools only for fallback/reference debugging, not as product contract.
- For Svelte, inspect generated API calls/events and bounded client state.

## Data reset

Provide explicit `just dev-reset` that destroys only labeled local data after confirmation/non-interactive flag. It must never target production endpoints based on ambient credentials.

## Offline/limited network

Cache approved dependencies/artifacts and local fixtures. Core fast tests should not depend on third-party websites. Real-site tests are separate and policy controlled.
