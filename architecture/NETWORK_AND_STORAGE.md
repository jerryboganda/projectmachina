---
title: "Network and Storage Architecture"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define the native network stack, policy enforcement, cookies, caches, storage APIs, proxies, and observability."
---

# Network and Storage Architecture

## Network goals

- Standards-oriented HTTP navigation and fetch behavior.
- HTTP/1.1 and HTTP/2 at P0; HTTP/3 when justified by workload evidence.
- Connection pooling, DNS/TLS reuse, decompression, redirects, cookies, caching, proxying, interception, and cancellation.
- Network policy enforcement before any connection.
- Bounded buffers and response sizes.
- Typed events and timing without leaking sensitive data.

## Request pipeline

```text
URL parse/normalize
 -> scheme and policy check
 -> DNS resolution and rebinding defense
 -> destination/IP policy check
 -> proxy selection/auth
 -> connection pool/TLS
 -> request headers/cookies
 -> optional interception
 -> response headers/redirect policy
 -> streaming body/decompression/byte budget
 -> parser/fetch consumer/cache
 -> timing and metering
```

## Policy checkpoints

Check both hostname and resolved addresses. Re-evaluate after redirects and DNS changes. Deny by default in the managed service:

- loopback, link-local, private, reserved, multicast, and metadata-service ranges unless explicitly permitted;
- `file:`, privileged local schemes, and unregistered custom schemes;
- unsafe ports and cross-tenant proxy reuse;
- oversized downloads and redirect loops.

## Fetch and XHR

Implement request/response objects, methods, headers, bodies/streams, redirects, credentials, CORS, preflight, abort signals, referrer policy, and error mapping according to prioritized standards tests. Avoid buffering full bodies when streaming is possible.

## Cookies

Use a centralized cookie jar per context/profile with domain/path, expiry, secure, HttpOnly, SameSite, partitioning, prefix, and public-suffix validation. Export/import through the state bridge preserves attributes and policy; values are never ordinary telemetry.

## Cache

Initial native cache may be memory-only and policy-bounded. Persistent cache is optional by profile. Cache keys include relevant request context. Cache correctness is more important than hit rate; unsupported cache semantics may disable caching rather than approximate silently.

## Web storage

- `localStorage`: origin-scoped and persistent only in persistent profiles.
- `sessionStorage`: context/page lifecycle as standards require.
- IndexedDB and Cache Storage: P1, implemented through versioned storage service interfaces.
- Service-worker storage/registration: isolated and capability-gated.

## Persistent profiles

Use encrypted-at-rest, tenant-scoped stores with explicit lifecycle, quota, locking, and migration. SQLite may back local/self-hosted profiles; managed service may use an abstract profile service plus encrypted object/volume storage. Never mount the same mutable profile concurrently without a defined locking model.

## Proxies

Support HTTP CONNECT and SOCKS as prioritized, with per-session credentials resolved from secret references. Proxy errors are typed. DNS-through-proxy behavior is explicit. Pooling never crosses incompatible tenant/proxy identities.

## Network interception

Expose request/response metadata and allow continue, modify, fulfill, or fail according to capability and policy. Apply size/time limits and mark behavior differences between native/CDP/BiDi clients.

## Resource policy

Profiles can block, metadata-load, or fully load classes such as images, fonts, media, stylesheets, scripts, frames, WebSockets, and workers. Blocking is surfaced as policy behavior, not a network failure.
