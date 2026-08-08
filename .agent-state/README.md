# Agent state

This directory stores committed state projections and task evidence summaries.
`scripts/agent/handoff.mjs` writes resumable JSON/Markdown handoffs and claim
transition projections under `evidence/`. Active claim records and lock data
live in the shared Git common directory (`.git/machina-claims`) for Git
worktrees; non-Git test roots use the ignored local fallback.
Claims, raw logs, secrets, traces, and build outputs remain ignored.
