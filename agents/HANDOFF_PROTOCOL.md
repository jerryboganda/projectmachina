---
title: "Agent Handoff Protocol"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Make every task safely resumable across sessions, models, and coding tools."
---

# Agent Handoff Protocol

## When to hand off

Write a handoff before:

- context compaction or session termination;
- tool/time/quota exhaustion;
- transfer to another coding product;
- changing from implementer to reviewer;
- waiting on a human or external dependency;
- abandoning or splitting a task.

## Handoff record

Use this exact structure in the task evidence directory or pull-request body:

```markdown
# Handoff — <task-id>

## Identity
- Task:
- Agent/tool:
- Branch:
- Worktree:
- Base commit:
- Current commit:
- Claim/lease:

## Objective and acceptance criteria
- ...

## Completed
- ...

## In progress
- File and exact state
- Uncommitted/WIP commit status

## Decisions and invariants
- ...

## Commands and results
- command: result; output artifact

## Failures and reproductions
- ...

## Changed files
- ...

## Remaining steps
1. ...

## Risks and blockers
- ...

## Recommended next action
- ...
```

## Handoff quality rules

- Use exact paths, commits, task IDs, and commands.
- State what is known versus inferred.
- Include reproduction steps for every active failure.
- Do not paste secrets, access tokens, sensitive page content, or oversized logs.
- Link artifacts and record hashes when evidence matters.
- Commit or intentionally discard local modifications; never leave ambiguous untracked work.

## Receiver protocol

The receiving agent must:

1. read `AGENTS.md` and current state;
2. verify the branch, commit, and worktree;
3. inspect the diff rather than trusting the prose;
4. reproduce the last relevant command;
5. confirm or renew ownership;
6. update the handoff if repository reality differs;
7. continue from the first incomplete acceptance condition.
