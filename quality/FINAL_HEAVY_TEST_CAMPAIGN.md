---
title: "Final Heavy Test and Certification Campaign"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the one consolidated exhaustive validation program run on the release candidate before GA."
---

# Final Heavy Test and Certification Campaign

## Entry criteria

- All required M0–M8 tasks merged.
- Release-candidate commit frozen except controlled repairs.
- Capability matrix generated from registry.
- No known critical/high functional/security finding.
- Test environments, corpora, client versions, reference browsers, and artifacts pinned.
- Reproducible build and deployment candidate available.
- Deferred-test ledger complete.

## Campaign tracks

### A — Build, installation, upgrade, and rollback

- Clean builds on supported architectures/OS targets.
- Reproducibility comparison and artifact hashes.
- Package/container installation and startup.
- Schema/data upgrade from supported prior version.
- Rollback and forward-recovery.
- SBOM, provenance, signature verification.

### B — Standards and native behavior

- Full prioritized WPT shards for HTML/DOM/navigation/events/forms/fetch/cookies/storage/frames/workers/WebSockets and implemented APIs.
- Known failures triaged and capability matrix consistent.
- Parser/DOM regression corpus and sanitizer runs.

### C — Differential compatibility

- Large deterministic fixture set and approved real-site/task corpus.
- Native vs pinned Chromium comparison of URL, lifecycle, DOM/semantic outcomes, cookies/storage, network, console/errors, and action postconditions.
- Hybrid success and fallback reason analysis.

### D — Protocol and client conformance

- HTTP/gRPC schema and backward compatibility.
- Pinned Playwright/Puppeteer matrix over CDP.
- Selenium/client matrix over BiDi.
- MCP specification/client interactions.
- SDK versions across supported languages.
- Disconnect, reconnect, cancellation, backpressure, and unsupported behavior.

### E — Security

- Authentication/authorization and cross-tenant tests.
- Sandbox escape-oriented negative tests and configuration audit.
- SSRF, DNS rebinding, redirect, proxy, scheme, and metadata controls.
- Secret canary through logs/traces/recordings/artifacts/model output.
- Workflow prompt-injection and high-impact approval tests.
- Dependency/container/IaC scans and independent penetration review.
- Incident and emergency-control tabletop/drill.

### F — Performance and capacity

- Native, hybrid, Chromium, and comparison products on controlled hardware.
- Cold/warm startup, task latency, CPU, memory, network, verified success, fallback, cost.
- Concurrency sweeps and saturation/queue fairness.
- Memory fragmentation and worker recycling.
- Svelte route/load/bundle performance.

### G — Reliability

- 24-hour broad soak and 72-hour selected production-like soak.
- Worker/process/browser crash injection.
- Network/storage/control-plane faults.
- Queue, retry, idempotency, cancellation, lease loss, and recovery.
- Rolling deploy, drain, autoscale, and circuit breakers.

### H — Data and disaster recovery

- Backup integrity and encrypted restore.
- Point-in-time recovery where promised.
- Object artifact recovery/expiry behavior.
- Region/service outage exercise and measured RTO/RPO.
- Retention/deletion verification.

### I — Frontend and developer experience

- Critical route end-to-end suite.
- Keyboard and screen-reader manual audit.
- Automated accessibility and security checks.
- Browser/device compatibility.
- SDK quick starts from clean environment.
- Docs links/examples/capability/error references.

## Suggested execution order

1. Build/install and static security checks.
2. Standards, differential, and conformance in parallel.
3. Security and performance on qualified build.
4. Load/soak/chaos and DR.
5. Frontend/DX and documentation verification throughout.
6. Evidence aggregation and independent review.

## Failure classification

Each failure gets stable ID, suite/shard, requirement/capability, reproducibility, severity, owner, disposition, repair commit, and rerun scope. Environment failures require evidence; they are not automatically product passes.

## Release gates

- Zero unresolved critical failures.
- Zero unresolved high security/cross-tenant/data-corruption failures unless authorized according to policy (GA should normally require zero).
- No silent unsupported behavior in certified surfaces.
- Hybrid/native success and reliability targets met or formally changed before claim.
- Benchmark methodology and public claims independently approved.
- DR/rollback and incident controls demonstrated.
- Capability matrix exactly matches evidence.

## Final report

Produce signed/hashed report containing build identity, environments, suites, raw artifact references, result summaries, failure dispositions, residual risk, capability matrix, benchmark report, security review, SLO readiness, rollback/DR results, and approvals.
