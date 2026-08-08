---
title: "Success Metrics and Measurement"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define product, technical, quality, and business indicators without rewarding misleading speed."
---

# Success Metrics and Measurement

## North-star metric

```text
Verified successful browser tasks
---------------------------------
CPU-seconds + memory-GB-seconds + retry cost + fallback cost
```

A task counts only when its declared postcondition is verified. Navigation completion alone is not success.

## Primary technical metrics

| Metric | Definition |
| --- | --- |
| Verified success rate | Successful verified tasks / attempted eligible tasks |
| Native fast-path rate | Successfully completed native tasks / hybrid attempts |
| Effective throughput | Verified tasks per core-hour and per memory-GB-hour |
| p50/p95/p99 task latency | End-to-end task duration by workload and engine |
| Fallback rate | Sessions/tasks migrated or routed to Chromium by reason |
| Retry amplification | Total attempts / final successful tasks |
| Crash-free sessions | Sessions without worker/runtime crash |
| Silent-unsupported count | Certified commands that appear successful but do not perform semantics; target zero |
| LLM token cost | Tokens used per discovered workflow and per recovery; normal replay target zero |

## Product metrics

- Time to first verified task.
- Active projects and repeat workflows.
- Workflow replay success without LLM.
- Capability/fallback report usage.
- Debug time from failure to classified root cause.
- Conversion from recorded session to approved workflow.

## Reliability and operations metrics

- SLO attainment and error-budget consumption.
- Queue wait and tenant fairness.
- Worker crash/recycle rates.
- Mean time to detect, classify, mitigate, and recover.
- Restore success and measured recovery point/time.
- Cost per 1,000 verified tasks by workload.

## Guardrail metrics

- Security incidents and policy violations.
- Egress blocks by category.
- Secrets detected in logs/traces/artifacts.
- Data-retention deletion success.
- Abuse reports and emergency blocks.
- Accessibility regressions.
- License/SBOM policy violations.

## Benchmark reporting rules

Every report includes hardware, software versions, network conditions, cache policy, wait condition, fidelity, process/isolation topology, concurrency, success definition, failures, retries, fallback, and raw reproducibility artifacts. Do not publish a multiplier based on unequal success or fidelity.
