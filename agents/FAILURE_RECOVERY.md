---
title: "Agent and Task Failure Recovery"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Recover safely from interrupted sessions, stale claims, failed repairs, broken branches, and partial merges."
---

# Agent and Task Failure Recovery

## Failure classes

| Class | Examples | Recovery |
| --- | --- | --- |
| Session interruption | context limit, network loss, tool crash | Resume from handoff and repository facts |
| Stale claim | no heartbeat after lease + grace | Inspect process/worktree, then reclaim |
| Build failure | compiler/type errors | Bounded targeted repair |
| Behavioral failure | acceptance test fails | Reproduce, minimize, repair or split defect |
| Review deadlock | repeated incompatible feedback | Escalate architecture question or assign adjudicator |
| Corrupt worktree | interrupted rebase, filesystem failure | Preserve patch, recreate from branch, verify diff |
| Partial merge | state/docs not updated | Reconcile in a dedicated recovery commit |
| External outage | registry, CI, cloud unavailable | Cache evidence, run local alternatives, mark external blocker |
| Security incident | secret exposure, suspected exploit | Stop affected work, follow incident response |

## Stale-claim recovery

An orchestrator may reclaim a task only after:

1. lease and grace have expired;
2. no active process or remote run is associated with the claim;
3. branch/worktree status is captured;
4. uncommitted work is preserved or intentionally discarded with evidence;
5. a recovery handoff is written;
6. ownership transfer is recorded atomically.

## Three-cycle repair limit

A repair cycle consists of one diagnosis, one focused change set, and one rerun of the failing gate. Repeating the same speculative edit does not count as progress. After three failed cycles:

- preserve the minimal reproduction;
- classify root cause uncertainty;
- split a research/spike task if useful;
- mark dependents blocked;
- continue unrelated ready tasks.

## Broken-main policy

A broken default branch is highest priority. Stop merges, assign one repair owner, revert the smallest offending change when a safe fix is not immediate, restore fast gates, and document root cause. Do not layer unrelated fixes onto a broken main branch.

## Rollback preference

Prefer reversible recovery:

1. revert merge;
2. disable feature flag;
3. route to Chromium fallback;
4. roll back deployment;
5. restore data from verified backup when necessary.

Never hide a failure by weakening an assertion, broadening a timeout without evidence, or marking a test flaky without an owner and expiry.
