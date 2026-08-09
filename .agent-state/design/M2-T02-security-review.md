# Threat Model / Safety Review — M2-T02 (Native URL/DNS/TLS/HTTP Streaming Loader)

> Produced by a wave-1 security research agent ahead of M2-T02 implementation.
> Read-only review; no code changes. Feed this directly into the M2-T02 builder prompt.

Scope: `crates/network` (currently empty — only `.gitkeep`, **not yet a workspace member**) and the destination-policy interface it must call into. Reviewed against `planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (§M2-T02), `planning/MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md` (§M3-T06), `architecture/NETWORK_AND_STORAGE.md`, `security/NETWORK_EGRESS.md`, `security/THREAT_MODEL.md`, `security/SECURITY_ARCHITECTURE.md`, `security/requirements.json`.

## 0. Governance gap found — fix as part of this task

- `crates/network/` is not in the root `Cargo.toml` `[workspace] members` yet.
- `crates/security-policy/` is also empty (`.gitkeep` only) — **no destination-classification/SSRF engine exists anywhere in the repo today.**
- `crates/policy/src/lib.rs` (the only populated policy crate) only has a free-text `egress_mode: String` field — no IP-range logic, no DNS-resolution hook, no redirect re-check. Cannot serve as the M2-T02 "network-policy callback" as-is.
- **`security/requirements.json` traceability gap:** `TM-03` (SSRF reaches metadata/internal service) and `TM-04` (DNS rebinding) map to `["M3-T06", "M7-T04"]` only — **M2-T02 is missing**, even though M2-T02's own acceptance criteria require rejecting private/invalid destinations. A reviewer trusting that file could sign off on a loader with zero SSRF defense. **Recommend the M2-T02 PR adds `"M2-T02"` to both entries' `tasks` arrays.**

The M2-T02 builder is standing up the project's first real SSRF-defense implementation, not wiring into an existing one.

## 1. SSRF defense

### Destination classes to reject by default
Loopback (incl. octal/hex/decimal-encoded `127.0.0.1`) · link-local `169.254.0.0/16`/`fe80::/10` · **cloud metadata** `169.254.169.254`, `169.254.170.2`, `fd00:ec2::254`, `100.100.100.200`, metadata.google.internal (block even via a public-looking hostname that *resolves* to these) · private RFC1918 · unique-local IPv6 `fc00::/7` · CGNAT `100.64.0.0/10` · reserved/special-use (`0.0.0.0/8`, TEST-NET ranges, `240.0.0.0/4`, `2001:db8::/32`, etc.) · multicast/broadcast · IPv4-mapped/NAT64 IPv6 tricks (`::ffff:127.0.0.1`, `64:ff9b::/96` — must decode embedded IPv4 and re-classify) · dangerous schemes (`file:`, `gopher:`, `dict:`) · unsafe/privileged ports (cross-protocol smuggling defense) · wildcard addresses.

### Where to enforce (pipeline)
1. Parse/normalize URL (canonical percent-encoding, IDNA, numeric-host forms; reject ambiguous hosts).
2. Pre-resolution policy check (catches literal-IP metadata URLs before any DNS call).
3. DNS resolution through an approved/pinned resolver (not the OS's ambient resolver invoked opaquely by the HTTP client).
4. Classify **every** resolved address (all A/AAAA records, not just the first).
5. Deny if **any** candidate violates policy — don't pick the first good one and ignore the rest.
6. **Bind the connection to the specific validated IP** — resolve-then-connect-to-IP, never resolve-then-connect-to-hostname (reopens the DNS-rebinding TOCTOU window). This is the crux of rebinding defense: classification and connect must use the same resolved IP with no second uncontrolled resolution in between.
7. Re-run the full check on **every redirect** and on any address change during connection reuse.
8. Meter/audit every policy decision (allow/deny + reason), independent of request success.

### Bypass techniques the negative-fixture suite must cover
Redirect chain to metadata IP · DNS rebinding (short-TTL second resolution) · mixed-answer DNS (one public + one bad IP) · alternate numeric IP encodings · IPv6-embedded-IPv4/NAT64 · userinfo confusion (`http://public@169.254.169.254/`) · open-redirect-as-SSRF-proxy · IPv6 zone-ID tricks.

## 2. Redirect handling

**Must be in M2-T02 (transport baseline):** redirect count limit (~20) + loop detection · total-time budget across the whole chain (not just per-hop) · cross-scheme (HTTPS→HTTP) downgrade must be a conscious, loggable decision · **every redirect re-enters the full SSRF pipeline above** · caller-supplied sensitive headers (e.g. `Authorization`) stripped by default on cross-origin (scheme/host/port change) redirect — this is a transport-layer credential-leak bug regardless of milestone ownership.

**Acceptable to defer to M3-T06 (log in BLOCKERS.md):** full cookie-jar credential-forwarding semantics (`SameSite`, domain/path scoping) · CORS-mode-aware credential inclusion · referrer-policy computation across redirects · cache-semantics interaction with redirects.

## 3. Response handling abuse — must be in M2-T02

- **Decompression bombs:** absolute decompressed-byte ceiling per response (hard backstop) + compression-ratio ceiling, enforced incrementally on the streaming decode path (not only at EOF); apply to chunked bodies and reject stacked/nested encodings unless explicitly required; reject on declared-vs-actual encoding mismatch rather than fallback-guessing.
- **Oversized responses:** never trust `Content-Length` alone (attacker-controlled/absent for chunked) — enforce both a pre-check and a live streamed-byte counter; cap aggregate header size/count/value length separately from the body budget; abort promptly on breach (don't keep draining).
- **Malformed headers / smuggling:** strict parser rejecting CR/LF injection, duplicate/conflicting `Content-Length`, simultaneous `Content-Length`+`Transfer-Encoding`, obsolete line-folding; never build outgoing request headers via naive string concatenation (typed builder that rejects embedded CR/LF).

## 4. TLS

**Hard-enforced, not configurable:** full chain + hostname validation on by default, no global "ignore cert errors" flag, no silent downgrade-to-plaintext on failure · reject expired/wrong-host/self-signed by default · TLS 1.2 floor (prefer 1.3), disable known-weak ciphers · cert failures are a distinct typed/auditable error class · validation happens on the actual connected IP (post-SSRF-classification), not just the hostname.

**Legitimately configurable, but scoped:** custom trusted CA bundle per project/session policy (never per-request from page/agent-controlled input) · certificate pinning for known internal targets (optional, later) · OCSP/CRL strictness (soft-fail-with-audit is a reasonable initial default — document it explicitly).

## 5. Resource exhaustion — must-have now

- Distinct connect / TLS-handshake / time-to-first-byte / idle-read timeouts (not one overall deadline) — defends against slow-loris.
- Minimum-throughput enforcement (abort if byte-rate drops below a floor past a grace window) + separate header-read timeout (attacker can trickle headers forever).
- Per-host and aggregate connection concurrency caps; cap DNS lookups in flight.
- Per-request and per-session byte budgets, both directions (upload too), cumulative across a session.
- Cancellation must actually tear down the socket/task, not just stop consuming it — interoperates with M2-T01's cancellation/deadline model.
- Backpressure on the streaming response API — bound internal buffer, pause reads at the socket level so a fast server + slow consumer can't self-inflict memory exhaustion.
- Expose an enforcement hook for tenant/session rate limits (per `security/NETWORK_EGRESS.md`); fail closed if the policy callback errors or is unreachable.

## 6. Pre-merge checklist

**[NOW]** = must be true before merge. **[DEFER→M3-T06]** = acceptable to leave, but must be explicitly logged in `agents/BLOCKERS.md`.

- [NOW] `crates/network` added to workspace members.
- [NOW] Concrete destination-policy module (address-class table above), not just the free-text `egress_mode` string.
- [NOW] `security/requirements.json` updated: add `M2-T02` to `TM-03`/`TM-04` task arrays.
- [NOW] Fast-gate fixtures exercise every class above plus the bypass techniques.
- [NOW] Pre-resolution + full-answer-set DNS classification + connect-to-classified-IP + per-redirect re-check, all implemented.
- [NOW] Redirect count cap + loop detection + chain-wide time budget + sensitive-header stripping cross-origin.
- [DEFER→M3-T06] Full cookie-jar/CORS-credential-aware redirect forwarding; referrer-policy computation.
- [NOW] Decompression-bomb ceilings enforced incrementally on the streaming path; `Content-Length` never trusted alone; header block caps; CR/LF-safe parsing; bodies genuinely stream.
- [NOW] TLS: full validation on by default, no bypass flag, TLS 1.2+ floor, custom-CA scoped to policy not page input.
- [NOW] Distinct per-phase timeouts, byte budgets, connection caps, real cancellation teardown, streaming backpressure, rate-limit enforcement hook (fail closed).
- [ ] PR records `[DEFER→M3-T06]` items in `agents/BLOCKERS.md`; independent security reviewer signs off separately from the implementer (CLAUDE.md review-separation rule).

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` (M2-T02) · `planning/MILESTONE_03_NATIVE_WEB_APIS_AND_AUTOMATION.md` (M3-T06) · `security/NETWORK_EGRESS.md` · `architecture/NETWORK_AND_STORAGE.md` · `security/THREAT_MODEL.md` (TM-03, TM-04, TM-09) · `security/SECURITY_ARCHITECTURE.md` · `security/requirements.json` · `crates/policy/src/lib.rs` · `crates/security-policy/.gitkeep` · `crates/network/.gitkeep` · root `Cargo.toml`.
