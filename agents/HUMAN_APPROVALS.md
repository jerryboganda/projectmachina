---
title: "Human Approval Gates"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the small set of decisions that autonomous agents may not make on behalf of the accountable owner."
---

# Human Approval Gates

## Principle

Agents continue automatically through reversible implementation. Human approval is reserved for accountability, irreversibility, privileged access, material risk, and public commitments.

## Gate catalog

| Gate | Trigger | Required approver | Evidence required |
| --- | --- | --- | --- |
| H-LEGAL-01 | Core/dependency license policy or unresolved copyleft effect | Legal/product owner | SBOM, dependency purpose, alternatives, counsel note |
| H-NAME-01 | Public product/company name | Business owner/legal | Name candidates and trademark search |
| H-CLOUD-01 | Production cloud account, region, or spend ceiling | Business/platform owner | Architecture, estimate, budget guardrails |
| H-SECRET-01 | Production secret or privileged credential | Security owner | Purpose, scope, expiry, storage, rotation |
| H-SEC-01 | Acceptance of high/critical security risk or weakened sandbox | Security owner | Threat, exploitability, mitigation, expiry |
| H-DATA-01 | Retention, residency, subprocessors, sensitive-data policy | Privacy/legal owner | Data map and policy proposal |
| H-RELEASE-01 | Beta, RC, or GA authorization | Product/release owner | Release report and open-risk register |
| H-CLAIM-01 | Public performance or compatibility claim | Independent technical owner | Reproducible benchmark/conformance evidence |
| H-PROD-01 | First production deploy or destructive migration | Platform owner | Plan, backup, rollback, rehearsal result |

## Approval request format

```markdown
> ### OWNER APPROVAL REQUIRED — <gate-id>
> **Decision:** ...
> **Recommended option:** ...
> **Alternatives:** ...
> **Why now:** ...
> **Impact of delay:** ...
> **Evidence:** ...
> **Rollback/reversibility:** ...
> **Select:** [A] ... [B] ... [C] ...
```

This is the portable popup format used in Markdown interfaces.

## Non-blocking behavior

When a gate blocks one task, the orchestrator marks only that task and its dependents blocked. It continues any ready work that does not presume the pending decision.

## Prohibited approval patterns

- No implicit approval through silence.
- No model may approve its own public claim or security waiver.
- No broad “approve all future actions” request.
- No bundling unrelated high-impact decisions into one gate.
- No secret values in an approval record.
