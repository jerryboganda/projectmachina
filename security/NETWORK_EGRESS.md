---
title: "Network Egress and SSRF Controls"
project: "Project Machina"
document_status: "approved-baseline"
version: "1.0.0"
last_updated: "2026-08-08"
owners: "Architecture and Program"
purpose: "Define destination policy, DNS defenses, proxies, redirects, rate limits, and emergency network controls."
---

# Network Egress and SSRF Controls

## Default managed-service policy

Allow public HTTP(S) destinations subject to organization/project policy; deny local, private, link-local, loopback, reserved, multicast, carrier-grade NAT, and cloud metadata networks. Deny non-HTTP schemes unless specifically supported and approved.

## Decision sequence

1. Parse and canonicalize URL.
2. Validate scheme, credentials, hostname syntax, and port.
3. Apply domain allow/deny policy.
4. Resolve DNS through approved resolver.
5. Classify all returned IP addresses.
6. Deny if any selected destination violates policy.
7. Connect through the session network namespace/proxy.
8. Bind/check destination at connection time.
9. Re-run checks on redirect, proxy tunnel, and DNS/address change.
10. Meter and audit policy-relevant outcomes.

## DNS rebinding

Do not trust a hostname allow decision after DNS resolution changes. Use resolution/connect binding, short caching consistent with TTL/policy, IP revalidation, and protection against mixed safe/unsafe answer selection.

## Redirects

Each redirect is a new policy decision. Limit count, total time, cross-scheme behavior, credential forwarding, and header/cookie semantics. Never forward authorization to a different origin unless standards/client policy explicitly permits.

## Proxies

Proxy references are authorized per project/session. Credentials are resolved at worker use and redacted. Define whether DNS resolves locally or via proxy. Do not pool tunnels across tenants or incompatible credentials.

## Rate and abuse controls

Per tenant/project/session/origin limits for concurrency, requests, bytes, new connections, DNS queries, WebSockets, and downloads. Honor `Retry-After` where applicable. Crawler profiles support robots and polite rate policies by default.

## WebSockets and long-lived channels

Validate initial destination and redirects, meter messages/bytes/time, cap connections, enforce idle/max lifetime, and prevent channels from outliving session authorization.

## Emergency controls

Audited domain/IP/ASN/category block, tenant suspension, feature kill, proxy revocation, and region egress shutdown. Changes propagate quickly to new requests and close existing channels when severity requires.

## Testing

Use controlled DNS/HTTP fixtures for private targets, rebinding, mixed answers, redirects, IPv4/IPv6 forms, encoded hosts, user-info confusion, alternative numeric notation, proxy behavior, and metadata endpoints.
