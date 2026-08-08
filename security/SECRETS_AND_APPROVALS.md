---
title: "Secrets, High-Impact Actions, and Approvals"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define secret-reference handling and policy-controlled human approval for consequential browser actions."
---

# Secrets, High-Impact Actions, and Approvals

## Secret model

Workflow definitions, API requests, recordings, task logs, and agent context contain references such as `secret://project/customer_portal_email`, never plaintext values.

## Secret lifecycle

1. Authorized user creates/links a secret through a secure channel.
2. Control plane stores metadata and vault reference, not a retrievable display value.
3. Workflow/session policy authorizes a specific reference and usage class.
4. Worker receives a scoped, expiring grant.
5. Value is resolved at the moment of use.
6. Value is passed directly to the target field/request as allowed.
7. Recording/log/trace stores reference and redaction marker.
8. In-memory value is released promptly; rotation/revocation is supported.

## Secret safeguards

- Never interpolate secrets into shell commands or URLs by default.
- Prevent console/page error capture from echoing known values.
- Redact exact values and common encoded forms where feasible.
- Seed canary secrets in testing and scan every artifact path.
- Do not return secret values through SDK/console after creation.
- Separate secret-management permission from workflow-run permission.

## High-impact action classes

- financial transaction or purchase;
- send message/email or external communication;
- publish/post content;
- delete account/data/resource;
- change password, MFA, permission, or security setting;
- upload sensitive file;
- submit regulated/sensitive form;
- accept legal terms or contract;
- disclose secret/personal data to a new origin.

## Approval policy

Policies may require `always`, `first-run`, `above-threshold`, `new-destination`, `changed-workflow`, or `never-allow`. Approval binds exact workflow version, run, step, action summary, destination, relevant redacted inputs, expiry, and approver.

## Approval card

```markdown
> ### ACTION APPROVAL REQUIRED
> **Workflow/run:** ...
> **Proposed action:** Send the completed form to example origin
> **Reason:** Policy `high-impact-submit`
> **Data categories:** contact information; no secret values displayed
> **Engine:** native/hybrid
> **Expiry:** ...
> **Select:** Approve once | Deny | Abort run
```

## Agent prompt-injection defense

Page content can suggest actions but cannot alter approval policy, tool permissions, secret scope, or destination allowlist. The orchestrator treats page instructions as untrusted observations and verifies intended user/workflow objective before action.

## Audit

Record requester, workflow/version/run/step, policy, decision, approver, timestamp, expiry, action result, and correlation IDs without secret values.
