# Claim and ownership helper

`claims.mjs` stores one JSON record per task under the shared Git common
directory (`.git/machina-claims/claims`) when the checkout is a repository, so
separate worktrees see the same claim store. Temporary non-Git test roots use
`.agent-state/claims/`.

Claims use repository-relative write globs and an atomic directory lock. Active
overlapping claims are rejected conservatively. A lease must pass its documented
grace period before recovery; heartbeat cannot revive an expired claim. An
operator must inspect the record and invoke `recover` with an actor and reason
before the path becomes available.

Examples:

```text
node scripts/agent/claims.mjs claim --task M0-T02 --agent platform-01 --branch agent/M0-T02-claims --worktree ../machina-worktrees/M0-T02-platform-01 --scope scripts/agent/**
node scripts/agent/claims.mjs heartbeat --task M0-T02 --agent platform-01
node scripts/agent/claims.mjs inspect
node scripts/agent/claims.mjs release --task M0-T02 --agent platform-01 --reason merged
```

If a process dies while holding the lock, wait for the stale-lock threshold,
inspect the repository, then use `recover-lock --actor ID --reason TEXT`.
