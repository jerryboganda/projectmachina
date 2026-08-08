# Claim, worktree, and handoff helpers

`claims.mjs` stores one JSON record per task under the shared Git common
directory (`.git/machina-claims/claims`) when the checkout is a repository, so
separate worktrees see the same claim store. Temporary non-Git test roots use
`.agent-state/claims/`.

Claims use repository-relative write globs and an atomic directory lock. Active
overlapping claims are rejected conservatively. A lease must pass its documented
grace period before recovery; heartbeat cannot revive an expired claim. An
operator must inspect the record and invoke `recover` with an actor and reason
before the path becomes available.

`task-registry.mjs` validates task IDs, dependency declarations, unknown
dependencies, cycles, and (when present) durable dependency completion status.
`worktree.mjs` performs real Git worktree create/inspect/remove operations and
never removes an unregistered or dirty worktree without an explicit force.
`handoff.mjs` writes the protocol-shaped JSON handoff plus Markdown and durable
task/claim evidence projections under `.agent-state/evidence/`.

Examples:

```text
node scripts/agent/claims.mjs claim --task M0-T02 --agent platform-01 --branch agent/M0-T02-claims --worktree ../machina-worktrees/M0-T02-platform-01 --scope scripts/agent/**
node scripts/agent/claims.mjs heartbeat --task M0-T02 --agent platform-01 --branch agent/M0-T02-claims --worktree ../machina-worktrees/M0-T02-platform-01
node scripts/agent/claims.mjs inspect
node scripts/agent/claims.mjs release --task M0-T02 --agent platform-01 --branch agent/M0-T02-claims --worktree ../machina-worktrees/M0-T02-platform-01 --reason merged
node scripts/agent/worktree.mjs create --task M0-T02 --branch agent/M0-T02-claims --worktree ../machina-worktrees/M0-T02-platform-01
node scripts/agent/worktree.mjs inspect --worktree ../machina-worktrees/M0-T02-platform-01
node scripts/agent/worktree.mjs remove --worktree ../machina-worktrees/M0-T02-platform-01
```

If a process dies while holding the lock, wait for the stale-lock threshold,
inspect the repository, then use `recover-lock --actor ID --reason TEXT`.
Stale lock recovery atomically fences the old token and records an audit; normal
claim operations never silently delete a stale lock.
