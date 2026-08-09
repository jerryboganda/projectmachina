# Design: M2-T02 — Native URL, DNS, TLS and HTTP Streaming Loader

> Produced by a wave-1 architect research agent. Read-only; no files changed.
> Pair with `.agent-state/design/M2-T02-security-review.md`.

## Key findings

- `Cargo.lock` has **no async/HTTP stack at all** yet (no tokio/hyper/reqwest/rustls) — this is the first crate introducing an async runtime; nothing displaced.
- `crates/network`/`crates/security-policy` empty, not in workspace `[members]` — add `crates/network` as part of this task.
- **Whole engine is synchronous today.** `EngineAdapter::execute` and `CommandContext` (crates/command-bus) are blocking, `Instant` deadline + `Arc<AtomicBool>` cancellation, no waker/executor. `crates/event-loop` (M2-T08) **depends on** M2-T02, not the reverse. So `network` must be self-contained with its own internal async runtime, expose a sync-callable API today, and expose a body type an async event loop can drive later without redesign.
- `CanonicalErrorCode` already has `InvalidUrl`/`NetworkPolicyBlocked`; `EventType` already has `NetworkRequestV1`/`NetworkResponseV1` — use these, don't invent parallel ones.
- `Session` already owns `ResourceBudget`/`ResourceUsage`; `network` must NOT depend upward on `machina-session` (native-core depends on network, not vice versa) — compose via a narrow trait instead (see §4).
- `MASTER_TASK_GRAPH.md`: M7-T04 is the *later*, separate SSRF/rebinding hardening task. M2-T02's job is the **mechanism** (correct call sites/data) + a conservative default policy; M7-T04 hardens policy *content* without changing the call graph.
- Existing loopback fixture server (`scripts/test/fixture-server.mjs`, `tests/fixtures/manifest.json`) is the right base to extend, not replace.
- D01 (clean-room, permissive-license target) rules out forking Chromium/Lightpanda source — not the use of standard permissive Rust crates (`url`, `hyper`, `rustls`), which is the expected approach.

## 1. Library/TLS choice

- **Async runtime: `tokio`**, privately owned by `crates/network` (small dedicated runtime, not workspace-wide).
- **HTTP client: `hyper` 1.x + `hyper-util`, not `reqwest`.** Redirects need hop-by-hop re-validation with full connector control (own `tower::Service<Uri>` connector that resolves→classifies→dials one specific address — closes the DNS-rebinding TOCTOU window `reqwest`'s `dns_resolver` hooks can't). Streaming bodies with per-chunk budget metering is `http_body::Body` directly. `reqwest` is built on `hyper` anyway — only its opinionated policy layer is what this design replaces.
- **TLS: `rustls` via `hyper-rustls`, not `native-tls`** — pure Rust, no OS-specific C TLS backend to vendor cross-platform, programmatic `ClientConfig`/`RootCertStore` for deterministic fixture-cert trust in tests. Default root store: `webpki-roots`; expose `TrustStoreSource` enum (`WebpkiRoots` default, `OsNativeCerts` optional) so enterprise-proxy roots is a deferred M7 decision, not an interface change.

## 2. URL normalization

Use the `url` crate (WHATWG URL Standard) — hand-rolling is explicitly wrong here per `NETWORK_EGRESS.md`'s own list of traps. Wrap in `NormalizedUrl` (`src/url.rs`). Required explicit test cases, all SSRF-bypass-relevant:
- Alternative IPv4 notations (octal/decimal/hex/short forms) — WHATWG `Host` parser canonicalizes these; **policy hook must only ever receive a canonical `IpAddr`**, never a raw host string — the one rule that prevents string-prefix-check bypass.
- Userinfo confusion (`http://expected.com@attacker.com/`) — policy input is the authority host, never userinfo; credentials in nav URLs stripped/flagged by default.
- IPv6 literals/zone-IDs (reject zone-IDs) and IPv4-mapped IPv6 (`::ffff:127.0.0.1`) — must classify as loopback/private.
- IDNA/Punycode — DNS resolution and audit logs use ASCII form, not raw Unicode (homograph bypass defense).
- Trailing dot (`example.com.`) treated identically to without.
- Non-network schemes (`data:`/`blob:`/`about:`/`file:`/`javascript:`) rejected at the loader boundary with `InvalidUrl`.
- Fragment stripped from wire request-target, retained on returned `NormalizedUrl` for navigation/history.

## 3. Connection pooling / streaming response

- **One `NetworkClient` (own pooled `hyper` client) per `Session`, not one global pool** — direct consequence of `NETWORK_EGRESS.md`: "do not pool tunnels across tenants or incompatible credentials." A global host:port-keyed pool would let one session's validated connection get reused by another with a different egress policy.
- Streaming shape (`src/response.rs`/`body.rs`): `ResponseHead{status, headers, http_version, final_url, redirect_chain}`; `ResponseBody` wraps `http_body::Body` + decompression + budget metering, exposes `next_chunk_blocking(ctx)` AND implements `http_body::Body` directly (forward-compatible with the future async event loop without a second body abstraction). Plus a `std::io::Read` blocking adapter.
- Backpressure by **not reading ahead** — no "read whole body into Vec" path in the primary API; only a narrowly-named, budget-capped `read_to_end_bounded(max_bytes)` helper for known-small payloads, still going through the same per-chunk budget check.

## 4. Deadline/cancellation/budget integration

- Deadline: `ctx.deadline: Instant` → `tokio::time::sleep_until`/`timeout` raced (`tokio::select!`) against connect, TLS handshake, header read, **and every individual body chunk read** — not just the initial request (defends against slow-loris after headers arrive).
- Cancellation: `CancellationToken::is_cancelled()` has no waker. **Recommend an additive, non-breaking `command-bus` change**: add `tokio::sync::Notify` alongside the existing `AtomicBool` so `cancel()` also calls `notify_waiters()` — flagged as a cross-crate decision for command-bus's contract owner, not silently forked. Fallback: poll on a short `tokio::time::interval` (25-50ms) inside the same `select!`.
- Budget: `network` defines its own narrow trait (does NOT depend on `machina-session`):
  ```rust
  pub trait RequestBudget: Send {
      fn reserve_request(&self) -> Result<(), NetworkError>;
      fn account_bytes(&self, n: u64) -> Result<(), NetworkError>;
  }
  ```
  `native-core` supplies an adapter over `Session` (needs a small additive `Session::account_bytes(&mut self, n: u64)` split out from today's all-at-once `account_request(bytes)` — small, out-of-crate, additive follow-up to flag). Client calls `reserve_request()` once, `account_bytes(n)` after every chunk, aborting the instant budget would be exceeded (not after buffering).
- Both deadline-exceeded and cancellation map 1:1 to `CanonicalErrorCode::DeadlineExceeded`/`CommandCancelled` — `network` is the **second, finer-grained enforcement point** during long-running I/O that the coarse pre-dispatch check in `CommandBus::execute_with_decision` can't reach.
- Timeout knobs: `connect_timeout`, `idle_read_timeout` (default 30s), both bounded by `ctx.deadline` as hard ceiling.

## 5. Network-policy callback hook

Mapped onto `NETWORK_EGRESS.md`'s decision sequence, scoped to M2-T02 vs M7-T04:
1. After URL normalization, before I/O: `evaluate_url` — scheme/syntax/embedded-credential checks.
2. After DNS resolution, before connect: `evaluate_resolution` — receives **every** candidate `IpAddr`, denies if any violates policy (mixed safe/unsafe answer set defense).
3. At connect time, with the single already-pinned `SocketAddr`: `evaluate_connect` — closes the TOCTOU window since the custom connector never lets anything re-resolve the hostname.
4. On every redirect: steps 1-3 rerun against the new target, as a loop around the same request function — never a separate weaker "redirect path."
5. Proxy-tunnel/WebSocket-upgrade re-checks deferred (M3-T10/M7-T04) but reuse the same trait method with a different `RequestMeta.purpose`, so no incompatible second interface later.

```rust
pub trait NetworkPolicy: Send + Sync {
    fn evaluate_url(&self, url: &NormalizedUrl, meta: &RequestMeta) -> PolicyDecision;
    fn evaluate_resolution(&self, host: &Host, addresses: &[IpAddr], meta: &RequestMeta) -> PolicyDecision;
    fn evaluate_connect(&self, address: SocketAddr, meta: &RequestMeta) -> PolicyDecision;
}
pub struct RequestMeta { session_id, tenant_id: Option<String>, purpose: RequestPurpose, redirect_depth: u32, correlation_id }
pub enum PolicyDecision { Allow, Deny { reason: DenyReason, message: String } }
```

All three methods synchronous/non-blocking (dynamic sources pre-fetched/cached by the implementer). `network` ships a conservative built-in default (`DenyPrivateNetworks`: RFC1918, loopback, link-local incl. `169.254.169.254`, CGNAT, multicast, IPv4-mapped IPv6, plus the §2 evasions) used as the fail-closed default and the base rules `security-policy` composes with — satisfies the acceptance criterion even before `security-policy` is populated. `Deny` → `CanonicalErrorCode::NetworkPolicyBlocked`, never silently downgraded. `crates/network` must NOT depend on `crates/security-policy` (dependency inverted — security-policy implements network's trait).

## 6. HTTP/1.1 + HTTP/2 scope

**In now:** HTTP/1.1 full cycle, keep-alive/pooling, chunked transfer, `Connection: close` (no pipelining). HTTP/2 TLS-ALPN-only, opportunistic, per-origin pooling reuses one H2 connection.
**Deferred:** H2 server push, HTTP/3/QUIC (no dep pinned); proxy support (M3-T10 — connector written as `tower::Service` so a proxying decorator layers in later); cookie jar (M2-T12, headers pass through opaquely); HTTP caching (M3-T06); DNS-rebinding-hardened resolver/cache + tenancy-aware proxy variants (M7-T04 — this task provides only the call sites).

## 7. Redirect/compression/chunked handling

- **Redirects handled manually in an explicit loop** (hyper never auto-follows). 303→GET+empty body; 301/302→rewrite POST to GET (legacy/Chromium behavior); 307/308→preserve method/body, fail typed `RedirectBodyNotReplayable` if a one-shot body was already consumed. `max_redirects` default 20. Strip `Authorization`/`Cookie`/`Proxy-Authorization` cross-origin; never forward cross-scheme downgrade regardless of origin match. Every hop reruns full policy + recorded in `redirect_chain`; each hop emits `network.request.v1`/`network.response.v1` via existing `CommandContext::record_trace` pattern.
- **Compression** (gzip/deflate/br) decoded as a streaming layer via `async-compression`'s Tokio `AsyncRead` decoders (avoids buffering compressed body first). Budget accounting applies to **decompressed** bytes plus a configurable max ratio (default 100x) as explicit zip-bomb control. Unrecognized/malformed `Content-Encoding` fails closed, never passed through as uncompressed.
- **Chunked transfer** entirely delegated to `hyper` (never hand-rolled), including its rejection of responses carrying both `Content-Length` and `Transfer-Encoding` (smuggling defense — kept, not relaxed).

## 8. Test strategy

Extends existing fixture infra (`tests/fixtures/manifest.json`, `scripts/test/fixture-server.mjs`) with new routes rather than parallel infra:
- Redirects/compression/chunking/cancellation → `/redirect-chain?n=` (same+cross-origin, credential-stripping check), `/compressed/{gzip,br,deflate}`, `/chunked`, `/slow-trickle?delay_ms=&chunks=` (client-cancel + deadline-exceeded mid-stream).
- Private/invalid destination rejection → table-driven unit tests against `DenyPrivateNetworks` for every class + normalization evasions; integration test with default policy pointed at fixture loopback address rejected before reaching handler (proves reject-by-default is the safety property under test); redirect-hop disguised-as-private negative fixture.
- Streaming without full buffering → 8 MiB body, `max_bytes` far below total, assert budget error fires at the chunk boundary that exceeds it (not after buffering); public API audit confirms no eager `Vec<u8>` path except the explicitly-named capped helper.
- Malformed-response fast gate → `/malformed/*` group + a minimal raw-TCP Rust fixture (for cases Node's http module won't produce): invalid `Content-Length`, both `Content-Length`+`Transfer-Encoding`, invalid chunk-size hex, truncated chunked body, garbage status line, oversized headers.

## 9. Module layout

```
crates/network/  Cargo.toml
  src/ lib.rs · url.rs (§2) · dns.rs (Resolver trait) · policy.rs (§5) ·
       connector.rs (tower::Service<Uri>: resolve→evaluate_resolution→pin IP→evaluate_connect→TCP→TLS) ·
       tls.rs (rustls ClientConfig, TrustStoreSource) · client.rs (NetworkClient, redirect loop) ·
       request.rs · response.rs · body.rs (budget metering) · compression.rs · budget.rs (RequestBudget trait) ·
       error.rs (NetworkError, mapping doc'd, mapping fn lives in native-core) · runtime.rs (internal tokio lifecycle)
  tests/ fixture_navigation.rs · policy_rejection.rs · malformed_responses.rs · url_normalization.rs
```

**Prerequisite workspace wiring:** add `"crates/network"` to root `[workspace] members`; new `Cargo.toml` with `tokio`/`hyper`/`hyper-util`/`hyper-rustls`/`rustls`/`webpki-roots`/`url`/`bytes`/`http`/`http-body`/`http-body-util`/`async-compression`, reusing already-pinned `serde`/`serde_json` versions. `crates/network` must NOT depend on `security-policy` or `session`. `native-core`'s dependency on `machina-network` is wired later (M2-T09), not here.

**Flagged cross-crate follow-ups (not this task's to silently fork):** `CancellationToken` gaining an optional `Notify`-based wakeup; `Session` splitting `account_request(bytes)` into `account_request()` + `account_bytes(n)`.

## Files reviewed

`planning/MILESTONE_02_NATIVE_ENGINE_FUNDAMENTALS.md` · `AGENTS.md` · `architecture/ADR-001-HYBRID_ENGINE.md` · `architecture/ADR-004-UNIFIED_COMMAND_BUS.md` · `OWNER_DECISIONS.md` (D01) · `security/NETWORK_EGRESS.md` · `quality/FAST_INNER_LOOP.md` · root `Cargo.toml`/`Cargo.lock` · `crates/command-bus/src/lib.rs` · `crates/native-core/src/lib.rs` · `crates/session/src/lib.rs` · `crates/policy/src/lib.rs` · `crates/command-model/src/generated.rs` · `tests/fixtures/manifest.json` · `scripts/test/fixture-server.mjs` · `planning/MASTER_TASK_GRAPH.md`.
