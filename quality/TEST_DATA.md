---
title: "Test Data, Corpora, Fixtures, and Privacy"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define safe, reproducible data sources for standards, differential, agent, security, and performance testing."
---

# Test Data and Corpora

## Categories

- WPT and upstream standards fixtures under their licenses.
- Project-owned deterministic local websites by capability.
- Synthetic hostile/security fixtures.
- Recorded network fixtures with permission and redaction.
- Approved real-site URL/task corpus with no stored credentials or prohibited data.
- Historical minimized regressions.
- Performance workload manifests.

## Fixture requirements

Every fixture has owner, purpose, license/provenance, data classification, expected behavior, stability strategy, version/hash, and retention. Deterministic fixtures should not depend on third-party Internet.

## Real-site corpus

Store task definitions and safe postconditions rather than captured personal content by default. Record observation date and site-change detection. Respect authorization, terms, robots/rate policy, and legal review. Do not include bypass/evasion tasks.

## Credentials

Use synthetic accounts and secret references. Never commit credentials. Reset or rotate test accounts. High-impact endpoints use local simulators unless production-like external testing is explicitly authorized.

## Privacy

Redact URLs/query/body/DOM/artifact content according to classification. Separate restricted corpus storage from source repository. Test outputs use pseudonymous IDs and bounded retention.

## Reproducibility

Pin fixture versions and serve local HTTPS origins with documented hostnames/certificates. A failure artifact identifies exact corpus/fixture version and seed.
