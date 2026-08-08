---
title: "Security Incident Response"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Provide a concise operational process for security detection, containment, investigation, recovery, and learning."
---

# Security Incident Response

## Severity

- **SEV-0:** active widespread compromise, signing/control-plane compromise, confirmed cross-tenant exposure, or uncontrolled secret exfiltration.
- **SEV-1:** confirmed exploitable isolation/egress/auth flaw with material exposure.
- **SEV-2:** contained security event or high-risk vulnerability without known broad exploitation.
- **SEV-3:** suspicious activity or lower-risk defect requiring tracked remediation.

## Immediate actions

1. Assign incident commander, security lead, operations lead, and recorder.
2. Preserve safe evidence and establish timeline/correlation IDs.
3. Contain: revoke credentials, block domains/IPs, drain workers, disable capability, roll back, isolate region/tenant.
4. Protect affected customers and prevent additional artifact/log exposure.
5. Avoid destructive cleanup until evidence needs are assessed.
6. Communicate on approved channels; do not place sensitive details in public issue/agent context.

## Investigation

Determine entry point, affected versions/tenants/data, exploit path, persistence, credentials, lateral movement, logs/artifacts integrity, and whether native/Chromium/control plane/supply chain is involved.

## Eradication and recovery

Patch/rebuild from trusted source, rotate credentials/keys, reimage workers, invalidate sessions/artifact URLs, verify tenant boundaries, canary under heightened monitoring, and restore capacity gradually.

## Notification

Legal/privacy/business owners determine customer, regulator, partner, or public notification. Agents may prepare facts but cannot make legal notification decisions.

## Post-incident

Within the approved period, publish an internal review with timeline, root cause, contributing factors, detection gaps, customer impact, remediation owners/dates, test additions, and architectural/process changes. Track actions to closure.

## Emergency commands

Document and rehearse:

- disable native engine or capability;
- force Chromium-only or native-only where safer;
- block destination/category;
- suspend credential/project/tenant;
- drain/terminate worker pool;
- revoke signing/release or session grants;
- restrict artifact access;
- roll back release.
