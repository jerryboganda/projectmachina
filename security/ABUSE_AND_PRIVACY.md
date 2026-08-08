---
title: "Responsible Use, Abuse Prevention, and Privacy"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define acceptable automation, crawling controls, privacy minimization, retention, and enforcement."
---

# Responsible Use, Abuse Prevention, and Privacy

## Responsible-use principles

Project Machina is for authorized automation, agents, extraction, and testing. It must not be designed or marketed to bypass access controls, anti-abuse systems, paywalls, rate limits, legal restrictions, or user consent.

## Platform controls

- Identity and project accountability.
- Per-tenant/origin quotas and rate limits.
- Destination policies and emergency blocks.
- Crawler profiles that support robots and polite scheduling by default.
- Audit and anomaly signals for scanning, credential abuse, excessive failures, or prohibited destinations.
- Terms/policy enforcement and incident escalation.
- Configurable user agent/contact where appropriate.

## Privacy by default

- Minimize collection; store metrics rather than page content.
- Content capture, screenshots, bodies, DOM snapshots, and recordings are off unless needed and authorized.
- Short default retention for diagnostic artifacts.
- Tenant-configurable retention within service/legal bounds.
- Verified deletion and artifact expiry.
- Region and subprocessor policies recorded before production commitments.
- Access to sensitive artifacts is explicit, audited, and least privilege.

## Data subject and tenant operations

Provide mechanisms, appropriate to service model, for export, deletion, retention policy, legal hold, credential revocation, and audit access. Document limitations where Project Machina is a processor executing customer-selected websites.

## Sensitive categories

Treat authentication data, financial/health/government identifiers, communications, precise location, children’s data, and unknown form contents conservatively. High-impact workflows require additional policy and may be unsupported without legal/compliance review.

## Abuse response

Detect → restrict affected key/project/tenant/destination → preserve minimal evidence → investigate → notify internal owners/customer as policy requires → remediate → restore cautiously → record lessons and control changes.

## Transparency

Document engine behavior, user agent options, rate controls, artifact capture, retention, and customer responsibilities. Do not claim robots compliance or privacy guarantees beyond implemented evidence.
