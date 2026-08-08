---
title: "Project Glossary"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Standardize terminology across product, architecture, protocols, agents, and test evidence."
---

# Project Glossary

| Term | Definition |
| --- | --- |
| Capability | A browser, protocol, isolation, or workflow behavior with explicit support status and evidence. |
| Capability router | Component that chooses native, Chromium, migration, or rejection based on policy and observed needs. |
| Certified | Verified against a declared test set, client/runtime version, and environment; not a universal claim. |
| Command bus | Internal typed behavior model invoked by all external adapters. |
| Compatibility engine | Managed Chromium runtime used for full fidelity and unsupported native capabilities. |
| Context | Isolated browser state container within a session, depending on engine and policy. |
| Differential test | Same workload executed in native and reference browser, with semantic outcomes compared. |
| Effective throughput | Verified successful tasks normalized by CPU, memory, retry, and fallback cost. |
| Fast gate | Small required per-task validation: formatting, compile/type, changed tests, contracts, focused smoke/security checks. |
| Fidelity profile | Policy controlling which resources and browser behaviors are loaded or approximated. |
| Final heavy campaign | Consolidated exhaustive M9 validation: broad WPT/differential/conformance, fuzz, load, soak, chaos, security, DR, and release rehearsal. |
| Native engine | Project Machina runtime implementing machine-relevant browser behavior without a full renderer. |
| Native fast path | Task completed entirely by the native engine. |
| Reproduction bundle | Redacted versioned evidence needed to reproduce a failure. |
| Semantic kernel | Logic deriving role, name, state, visibility, interactability, order, and machine-oriented structure from DOM/CSS state. |
| Session | Top-level scheduled execution resource with policy, lifecycle, engine state, quotas, and telemetry. |
| State bridge | Contract and mechanisms for moving transferable session state and replaying verified actions between engines. |
| Task-ready | A workload-specific condition indicating the browser can perform the next action, not necessarily full page load. |
| Verified action | Interaction with checked precondition and postcondition. |
| Workflow | Versioned deterministic program produced manually or from a recorded successful session. |
| Worktree | Isolated Git checkout used by one implementation task/agent. |
