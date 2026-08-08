---
title: "Continuous Integration and Delivery"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define fast pull-request gates, scheduled suites, release pipelines, security controls, and deployment promotion."
---

# Continuous Integration and Delivery

## Pull-request pipeline

Target a fast deterministic gate:

1. repository policy, path-claim, and generated-file checks;
2. format/lint/static analysis for changed areas;
3. compile/type-check changed packages and direct dependents;
4. selected unit/component/contract tests;
5. task-focused smoke and risk-specific security check;
6. docs/link/schema compatibility;
7. secret/dependency/license diff scan;
8. evidence summary posted to PR.

Long broad suites are not required on each PR unless the change is release-critical or explicitly high risk.

## Main/scheduled pipeline

- Build all main artifacts.
- Selected integration/e2e.
- Rotating WPT and differential shards.
- Protocol/client canaries.
- Short fuzz/sanitizer targets.
- Security baseline and container/IaC scans.
- Performance smoke and bundle budgets.
- State/claim cleanup and documentation drift checks.

## Milestone/release-candidate pipeline

Broader suites described in M8 and M9. Use a frozen release candidate, immutable artifacts, parallel test tracks, and central evidence registry.

## Security boundaries

- Untrusted PR jobs receive no production/signing/deployment secrets.
- Release/signing jobs use protected environment and approved commit/tag.
- Prefer OIDC/workload identity to long-lived cloud keys.
- Actions/plugins/images are pinned by immutable version/digest and reviewed.
- Artifacts have retention/classification and access policy.

## Promotion

```text
commit -> PR fast gate -> main artifact
 -> development -> canary -> beta/staging -> production
```

Promote the same immutable image/binary, not a rebuild. Environment configuration is versioned separately and recorded.

## Deployment gates

- health/readiness and smoke;
- migration compatibility;
- error/SLO/resource canary thresholds;
- no critical/high security block;
- rollback ready;
- human approval for first production, RC/GA, or destructive data migration.

## Failure behavior

A failed main gate blocks new merges until classified. A failed canary automatically halts/rolls back where safe. CI retries only proven infrastructure-flaky steps and exposes both first and retry results.
