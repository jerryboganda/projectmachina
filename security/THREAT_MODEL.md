---
title: "Threat Model"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Enumerate assets, actors, entry points, abuse cases, trust boundaries, and required mitigations."
---

# Threat Model

## Assets

- Tenant credentials, cookies, storage, form values, workflow inputs, outputs, and artifacts.
- Secret vault values and proxy credentials.
- Control-plane identities, policies, billing/usage, and audit data.
- Worker hosts, network access, cloud identity, and deployment credentials.
- Build/signing infrastructure and source integrity.
- Capability, benchmark, and release evidence.

## Threat actors

- Malicious website/page author.
- Malicious or compromised tenant/user/API key.
- Cross-tenant attacker.
- Supply-chain attacker.
- External network attacker.
- Compromised worker/container.
- Insider with excessive privilege.
- Prompt-injection content manipulating an AI agent.
- Accidental operator or coding-agent mistake.

## Entry points

Public APIs, protocol WebSockets, MCP, console, SDK input, URLs, proxy configuration, HTTP responses, HTML, JavaScript/WebAssembly, WebSockets, workers, downloads/uploads, storage import, workflow definitions, artifacts, dependencies, CI inputs, and administrative operations.

## Threat scenarios and mitigations

| ID | Scenario | Primary mitigations |
| --- | --- | --- |
| TM-01 | Native parser/DOM memory corruption | Rust, bounded parsing, fuzzing, sanitizers, process/container isolation |
| TM-02 | V8/Chromium exploit escapes page context | timely updates, process/hardened tiers, seccomp/namespaces/microVM, no host credentials |
| TM-03 | SSRF reaches metadata or internal service | DNS/IP validation before and after resolution/redirect, deny ranges, network namespace/firewall |
| TM-04 | DNS rebinding bypasses hostname allowlist | resolve and pin/validate each connection, recheck redirects/address changes |
| TM-05 | Cross-tenant session/artifact access | resource-scoped authorization, non-guessable IDs, tenant filters, tests/audit |
| TM-06 | Secret appears in trace/recording/model context | opaque references, centralized redaction, canary scanning, restricted capture |
| TM-07 | Page prompt injection induces unsafe tool call | channel separation, fixed tool policy, action allowlist/approval, no page authority |
| TM-08 | Replayed migration repeats purchase/send/delete | verified checkpoints, side-effect classification, no unsafe automatic replay |
| TM-09 | Malformed protocol exhausts memory/CPU | schema/size/rate limits, bounded queues, deadlines, connection quotas |
| TM-10 | Tenant uses platform for scanning/abuse | identity, quotas, egress/rate policy, abuse detection, emergency block |
| TM-11 | Download/upload accesses host filesystem | virtual handles, isolated ephemeral volume, path canonicalization, policy/scanning |
| TM-12 | Compromised dependency/build injects code | pinning, review, provenance, reproducible builds, signed artifacts, SBOM |
| TM-13 | Agent edits security controls to make tests pass | protected paths, mandatory security reviewer, policy tests, human risk gate |
| TM-14 | Logs leak URL/query/header/page data | data classification, allowlisted fields, redaction, sampling/retention |
| TM-15 | Shared worker leaks state between sessions | generational handles, context reset, isolation tests, dedicated default for untrusted users |
| TM-16 | Stale worker grant accesses control plane | short-lived scoped grants, audience/session binding, revocation/rotation |
| TM-17 | Artifact URL shared externally | short-lived signed URL, tenant auth at issuance, encryption, audit |
| TM-18 | Workflow definition injects arbitrary host code | typed DSL, sandboxed expressions, capability policy, signing/version/approval |

## STRIDE summary

- Spoofing: strong auth, signed grants, TLS, credential rotation.
- Tampering: signed artifacts/releases, hashes, immutable workflow versions, audit.
- Repudiation: correlation, approval and privileged-action audit.
- Information disclosure: tenant isolation, redaction, encryption, minimization.
- Denial of service: quotas, budgets, backpressure, isolation, circuit breakers.
- Elevation of privilege: least privilege, sandbox, policy checks, JIT admin.

## Threat-model maintenance

Update when adding a trust boundary, protocol, storage class, native Web API, isolation tier, secret path, deployment topology, or privileged operation. Link mitigations to tasks and tests.
