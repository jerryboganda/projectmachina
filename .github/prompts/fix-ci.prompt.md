---
mode: agent
description: Repair focused CI failure
---

# Repair focused CI failure

Reproduce the failing check from its exact commit and environment, identify the first causal failure rather than downstream noise, make the narrowest repair inside the task scope, run the failed check and directly affected fast gates, and update the evidence record. Do not mask, skip, quarantine, or broaden timeouts without documented approval.
