---
title: "Antigravity Autonomous Build Workflow"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Agentic Development and Platform"
purpose: "Provide an executable, resumable orchestration workflow for continuous Project Machina development."
---

# Antigravity Autonomous Build Workflow

## Invocation

Use a workspace workflow such as `/machina-build` to execute the cycle. The workflow is a controller around bounded agent runs; it must not depend on one model context remaining alive.

## State machine

```text
BOOT
  -> RECONCILE
  -> SELECT_READY
  -> CLAIM
  -> PREPARE
  -> DISPATCH_IMPLEMENTER
  -> FAST_GATE
  -> DISPATCH_REVIEWER
  -> REPAIR? (maximum 3)
  -> MERGE_QUEUE
  -> CHECKPOINT
  -> SELECT_READY

Any state -> HANDOFF on context/lease shutdown
Any task -> BLOCKED after proven external blocker or exhausted repairs
Program -> FINAL_FREEZE -> M9 -> HUMAN_RELEASE_GATE
```

## Controller pseudocode

```text
while program_state not in {complete, externally_stopped}:
    reconcile_repository_and_provider_state()
    expire_only_proven_stale_claims()
    schedule_up_to(config.max_implementation_agents)

    for slot in free_slots:
        task = highest_priority_ready_non_overlapping_task()
        if task is None:
            break
        claim = atomic_claim(task, slot.agent, task.write_scope)
        if not claim.ok:
            continue
        env = prepare_isolated_worktree_and_environment(claim)
        result = run_specialist(task.role, env, task.packet)
        persist(result.handoff_and_artifacts)

        if result.transient_failure:
            bounded_infrastructure_retry()
            continue
        if not result.fast_gate_passed:
            bounded_repair_or_block()
            continue

        review = run_independent_reviewer(result)
        for cycle in 1..3:
            if review.blocking_findings.empty:
                break
            result = run_repair_agent(review.blocking_findings, result)
            review = run_independent_reviewer(result)
        if review.blocking_findings:
            block_task_and_continue_independent_work()
            continue

        enqueue_merge_in_dependency_order()
        after_merge_update_state_evidence_capabilities()
        release_claim_and_destroy_or_archive_worktree()

    if all_M0_to_M8_required_tasks_complete:
        freeze_candidate_and_execute_M9()
```

## Default concurrency

- two implementation lanes;
- one independent reviewer lane that may review either implementation lane;
- serialized merge queue for shared contracts and lockfiles;
- security reviewer added for high-risk tasks;
- no more concurrency until M0 proves claim, worktree, merge, and recovery automation.

## Provider/environment policy

Each run receives only its worktree, task packet, approved network access, time/resource ceiling, and task-specific secrets. Production credentials are absent. Environment setup is reproducible and recorded by image/toolchain digest.

## Liveness

A running task writes a heartbeat. Missing heartbeat does not immediately release ownership. The controller checks provider status, process/worktree state, unpushed commits, pull requests, and handoff. It either resumes, hands off, or safely reclaims.

## Completion condition

The loop ends only when all required M0–M9 tasks are complete or approved waivers remove the affected public capability, the final candidate passes release gates, and a human accountable owner authorizes GA. An individual failed run is not program completion and is not a reason to discard durable progress.
