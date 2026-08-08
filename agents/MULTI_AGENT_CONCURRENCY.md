---
title: "Multi-Agent Concurrency and Worktree Protocol"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Enable two or more tools to work safely without overlapping edits or hidden coordination."
---

# Multi-Agent Concurrency and Worktree Protocol

## Default topology

Run at most two implementation agents concurrently until M1 telemetry shows fewer than five percent of tasks experience ownership or merge collision. Reviewer agents may run additionally but remain read-only unless assigned a repair task.

## Worktree requirement

Each active task uses:

```text
../machina-worktrees/<task-id>-<agent-id>/
branch: agent/<task-id>-<short-slug>
```

Never run two editing agents in the same checkout. Never use an uncommitted main working tree as a shared scratch space.

## Claims

A claim is represented conceptually as:

```json
{
  "task_id": "M2-T04",
  "agent_id": "claude-native-01",
  "branch": "agent/M2-T04-html-parser",
  "worktree": "../machina-worktrees/M2-T04-claude-native-01",
  "write_scope": ["crates/html/**", "tests/html/**"],
  "contract_dependencies": ["crates/contracts/src/navigation.rs"],
  "claimed_at": "RFC3339",
  "lease_expires_at": "RFC3339",
  "heartbeat_at": "RFC3339"
}
```

The implementation created in M0 stores atomic claims under `.agent-state/claims/` and enforces them through the agent CLI and CI. Before that implementation exists, use one GitHub issue per task with an `agent:claimed` label and explicit path list.

## Ownership rules

- A path may have one write owner.
- Parent-directory ownership includes descendants unless exclusions are explicit.
- Shared schema, lockfile, root build, CI, generated source definitions, and migration directories are serialized.
- Generated outputs inherit ownership from the source definition; another agent must not regenerate them concurrently.
- Documentation updates local to a subsystem belong to the subsystem task. Root indices are updated by the integrator.
- Read-only research has no path claim but may not edit implementation files.

## Contract-first sequencing

When two tasks depend on a new interface:

1. Create a small contract task or designate one contract owner.
2. Merge the interface, fixtures, and compatibility notes first.
3. Rebase dependent worktrees.
4. Implement consumers independently.

Do not allow both agents to invent their own versions and reconcile at the end.

## Recommended tool pairings

| Pair | Pattern |
| --- | --- |
| Claude Code + Codex | Claude handles architecture/native slice; Codex handles protocol/test slice; swap reviewer roles |
| Antigravity + Claude Code | Antigravity orchestrates task queue; Claude executes bounded worktree tasks |
| Antigravity + Codex | Antigravity mission control; Codex local/cloud workers |
| Copilot coding agent + Claude/Codex | Copilot handles issue-to-PR independent tasks; local agent reviews or handles critical path |
| AI Studio managed agent + local tool | Managed agent performs bounded independent implementation; local tool integrates and reviews |

Tool identity never substitutes for ownership. Two products can still conflict if they edit the same scope.

## Merge-conflict policy

1. Stop edits in the conflicted worktree.
2. Determine whether the conflict is semantic or textual.
3. The owner of the earlier dependency resolves shared-contract conflicts.
4. The later task rebases and re-runs its fast gate.
5. Never use blanket `ours`/`theirs` for source, schema, tests, or security configuration.
6. Record repeated collision as a task-graph or component-boundary defect.

## Cross-tool handoff

Any supported tool may take over a task only after:

- the current owner writes a handoff;
- uncommitted changes are committed to a clearly marked WIP commit or exported as a patch;
- claim ownership is transferred atomically;
- the new tool verifies base commit, branch, changed paths, and outstanding findings.

Chat transcripts are optional context, never the handoff source of truth.
