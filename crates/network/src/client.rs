//! `NetworkClient`: one pooled-connection-capable client per `Session`
//! (design section 3 -- never a global cross-session pool, per
//! `NETWORK_EGRESS.md`'s "do not pool tunnels across tenants or
//! incompatible credentials"). This pass connects fresh per hop rather than
//! reusing connections across separate `fetch` calls; see the evidence file
//! for the explicit scope note. The manual redirect loop below is the
//! single place every hop re-enters the full SSRF policy pipeline -- there
//! is no separate, weaker "redirect path".

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use http_body_util::Full;
use hyper_util::rt::{TokioExecutor, TokioIo};
use machina_command_bus::CommandContext;
use rustls::ClientConfig;

use crate::body::{ResponseBody, ResponseLimits};
use crate::budget::RequestBudget;
use crate::connector::{connect, AsyncStream};
use crate::dns::Resolver;
use crate::error::NetworkError;
use crate::policy::{NetworkPolicy, PolicyDecision, RequestMeta};
use crate::request::RequestSpec;
use crate::response::{RedirectHop, ResponseHead};
use crate::runtime::{guarded, NetworkRuntime};
use crate::url::{host_key, NormalizedUrl};

const SENSITIVE_HEADERS: [&str; 3] = ["authorization", "cookie", "proxy-authorization"];

#[derive(Clone, Debug)]
pub struct ClientConfigOptions {
    pub max_redirects: u32,
    pub connect_timeout: Duration,
    pub idle_read_timeout: Duration,
    pub response_limits: ResponseLimits,
}

impl Default for ClientConfigOptions {
    fn default() -> Self {
        Self {
            max_redirects: 20,
            connect_timeout: Duration::from_secs(10),
            idle_read_timeout: Duration::from_secs(30),
            response_limits: ResponseLimits::default(),
        }
    }
}

pub struct NetworkClient {
    runtime: NetworkRuntime,
    resolver: Arc<dyn Resolver>,
    policy: Arc<dyn NetworkPolicy>,
    budget: Arc<dyn RequestBudget>,
    tls_config: Arc<ClientConfig>,
    options: ClientConfigOptions,
}

impl NetworkClient {
    pub fn new(
        resolver: Arc<dyn Resolver>,
        policy: Arc<dyn NetworkPolicy>,
        budget: Arc<dyn RequestBudget>,
        tls_config: Arc<ClientConfig>,
        options: ClientConfigOptions,
    ) -> Result<Self, NetworkError> {
        Ok(Self {
            runtime: NetworkRuntime::new()?,
            resolver,
            policy,
            budget,
            tls_config,
            options,
        })
    }

    /// Handle onto the crate's internally-owned runtime, used to drive a
    /// `ResponseBody`'s blocking read methods after `fetch` returns.
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle()
    }

    /// Synchronous facade over the crate's internally-owned async runtime
    /// (see `runtime.rs`): the whole engine is synchronous today, so this
    /// is the entry point native-core calls. The returned `ResponseBody`
    /// still streams -- only the request phase (through response headers)
    /// is a blocking call from the caller's perspective.
    pub fn fetch(
        &self,
        spec: RequestSpec,
        session_id: impl Into<String>,
        ctx: &CommandContext,
    ) -> Result<(ResponseHead, ResponseBody), NetworkError> {
        let session_id = session_id.into();
        self.runtime
            .block_on(self.fetch_async(spec, session_id, ctx.clone()))
    }

    async fn fetch_async(
        &self,
        mut spec: RequestSpec,
        session_id: String,
        ctx: CommandContext,
    ) -> Result<(ResponseHead, ResponseBody), NetworkError> {
        let mut redirect_chain: Vec<RedirectHop> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut redirect_depth: u32 = 0;

        loop {
            if !visited.insert(spec.url.as_str().to_owned()) {
                return Err(NetworkError::RedirectLoop);
            }

            self.budget.reserve_request()?;

            let meta = RequestMeta {
                session_id: session_id.clone(),
                tenant_id: None,
                purpose: crate::policy::RequestPurpose::Navigation,
                redirect_depth,
                correlation_id: ctx.correlation_id.clone(),
            };

            enforce(self.policy.evaluate_url(&spec.url, &meta))?;

            let host = spec.url.host_owned();
            let port = spec.url.port();
            let tls_config = spec.url.is_https().then(|| self.tls_config.clone());

            let connected = guarded(
                &ctx,
                connect(
                    self.resolver.as_ref(),
                    self.policy.as_ref(),
                    &host,
                    port,
                    tls_config,
                    &meta,
                    self.options.connect_timeout,
                ),
            )
            .await?;

            let (head, incoming) = self
                .send_one_request(&spec, connected.stream, connected.alpn.as_deref(), &ctx)
                .await?;

            if is_redirect_status(head.status) {
                let Some(location) = head
                    .headers
                    .get(http::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                else {
                    // No usable Location header: return the redirect
                    // response as-is rather than guessing.
                    return Ok((
                        ResponseHead {
                            status: head.status,
                            headers: head.headers,
                            http_version: head.http_version,
                            final_url: spec.url.clone(),
                            redirect_chain,
                        },
                        ResponseBody::from_incoming(
                            incoming,
                            head.headers_content_encoding.as_deref(),
                            self.options.response_limits,
                            self.budget.clone(),
                            ctx.clone(),
                            self.options.idle_read_timeout,
                        )?,
                    ));
                };

                redirect_chain.push(RedirectHop {
                    url: spec.url.clone(),
                    status: head.status,
                });
                redirect_depth += 1;
                if redirect_depth > self.options.max_redirects {
                    return Err(NetworkError::TooManyRedirects);
                }

                let next_url = NormalizedUrl::parse(location, Some(&spec.url))
                    .map_err(|error| NetworkError::InvalidRedirect(error.to_string()))?;

                let previous_origin = spec.url.origin_tuple();
                let next_origin = next_url.origin_tuple();
                let cross_origin = previous_origin != next_origin;
                let downgrade = spec.url.is_https() && !next_url.is_https();

                let (next_method, next_body) =
                    rewrite_for_redirect(&spec.method, head.status, spec.body.clone());

                let mut next_headers = spec.headers.clone();
                if cross_origin || downgrade {
                    for name in SENSITIVE_HEADERS {
                        next_headers.remove(name);
                    }
                }

                spec = RequestSpec::new(next_method, next_url, next_headers, next_body);
                continue;
            }

            return Ok((
                ResponseHead {
                    status: head.status,
                    headers: head.headers.clone(),
                    http_version: head.http_version,
                    final_url: spec.url.clone(),
                    redirect_chain,
                },
                ResponseBody::from_incoming(
                    incoming,
                    head.headers_content_encoding.as_deref(),
                    self.options.response_limits,
                    self.budget.clone(),
                    ctx.clone(),
                    self.options.idle_read_timeout,
                )?,
            ));
        }
    }

    async fn send_one_request(
        &self,
        spec: &RequestSpec,
        stream: Box<dyn AsyncStream>,
        alpn: Option<&[u8]>,
        ctx: &CommandContext,
    ) -> Result<(RawHead, hyper::body::Incoming), NetworkError> {
        let io = TokioIo::new(stream);
        let use_h2 = alpn == Some(b"h2");
        let mut request = build_request(spec)?;
        *request.version_mut() = if use_h2 {
            http::Version::HTTP_2
        } else {
            http::Version::HTTP_11
        };

        // HTTP/2 is only ever used when the TLS peer opportunistically
        // negotiated it via ALPN during the connector's handshake (design
        // section 6); plaintext HTTP always uses HTTP/1.1 (H2C is out of
        // scope for this pass, tracked in the evidence file, not silently
        // downgraded -- HTTP/1.1 is a fully compliant fallback every origin
        // in the fixtures supports).
        let mut sender = if use_h2 {
            let handshake = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
                .handshake::<_, Full<Bytes>>(io);
            let (sender, connection) = guarded(ctx, async {
                handshake
                    .await
                    .map_err(|error| NetworkError::ProtocolError(error.to_string()))
            })
            .await?;
            tokio::spawn(async move {
                let _ = connection.await;
            });
            ConnSender::Http2(sender)
        } else {
            let handshake = hyper::client::conn::http1::Builder::new()
                .max_headers(self.options.response_limits.max_header_count)
                .max_buf_size(self.options.response_limits.max_header_bytes.max(8192))
                .handshake::<_, Full<Bytes>>(io);
            let (sender, connection) = guarded(ctx, async {
                handshake
                    .await
                    .map_err(|error| NetworkError::ProtocolError(error.to_string()))
            })
            .await?;
            tokio::spawn(async move {
                let _ = connection.await;
            });
            ConnSender::Http1(sender)
        };

        let response = guarded(ctx, async {
            sender
                .send_request(request)
                .await
                .map_err(|error| NetworkError::ProtocolError(error.to_string()))
        })
        .await?;

        let (parts, incoming) = response.into_parts();
        // Defense in depth: `hyper` itself already rejects some conflicting
        // framing at the wire-parse layer, but does not treat a response
        // that carries *both* `Content-Length` and `Transfer-Encoding`
        // headers as invalid on its own (it prioritizes
        // `Transfer-Encoding`). A request-smuggling-shaped response like
        // that must still fail closed here rather than being silently
        // accepted under hyper's interpretation.
        if parts.headers.contains_key(http::header::CONTENT_LENGTH)
            && parts.headers.contains_key(http::header::TRANSFER_ENCODING)
        {
            return Err(NetworkError::ProtocolError(
                "response carries both Content-Length and Transfer-Encoding".to_owned(),
            ));
        }
        let content_encoding = parts
            .headers
            .get(http::header::CONTENT_ENCODING)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        Ok((
            RawHead {
                status: parts.status,
                headers: parts.headers,
                http_version: parts.version,
                headers_content_encoding: content_encoding,
            },
            incoming,
        ))
    }
}

struct RawHead {
    status: StatusCode,
    headers: HeaderMap,
    http_version: http::Version,
    headers_content_encoding: Option<String>,
}

enum ConnSender {
    Http1(hyper::client::conn::http1::SendRequest<Full<Bytes>>),
    Http2(hyper::client::conn::http2::SendRequest<Full<Bytes>>),
}

impl ConnSender {
    async fn send_request(
        &mut self,
        request: http::Request<Full<Bytes>>,
    ) -> Result<http::Response<hyper::body::Incoming>, hyper::Error> {
        match self {
            Self::Http1(sender) => sender.send_request(request).await,
            Self::Http2(sender) => sender.send_request(request).await,
        }
    }
}

fn enforce(decision: PolicyDecision) -> Result<(), NetworkError> {
    match decision {
        PolicyDecision::Allow => Ok(()),
        PolicyDecision::Deny { reason, .. } => Err(NetworkError::PolicyBlocked(reason)),
    }
}

fn is_redirect_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

/// 303 always becomes GET with an empty body. 301/302 rewrite POST to GET
/// with an empty body (legacy/Chromium-compatible behavior) and otherwise
/// preserve the method. 307/308 always preserve method and body -- request
/// bodies in this crate are pre-buffered `Bytes` (see `request.rs`), so
/// replay never fails with a one-shot-body-consumed error.
fn rewrite_for_redirect(method: &Method, status: StatusCode, body: Bytes) -> (Method, Bytes) {
    match status {
        StatusCode::SEE_OTHER => (Method::GET, Bytes::new()),
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND => {
            if *method == Method::POST {
                (Method::GET, Bytes::new())
            } else {
                (method.clone(), body)
            }
        }
        _ => (method.clone(), body),
    }
}

fn build_request(spec: &RequestSpec) -> Result<http::Request<Full<Bytes>>, NetworkError> {
    let mut headers = spec.headers.clone();
    let host_header_value = host_header(spec);
    headers.insert(
        http::header::HOST,
        HeaderValue::from_str(&host_header_value)
            .map_err(|error| NetworkError::ProtocolError(error.to_string()))?,
    );

    let mut builder = http::Request::builder()
        .method(spec.method.clone())
        .uri(spec.url.as_str())
        .version(http::Version::HTTP_11);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    builder
        .body(Full::new(spec.body.clone()))
        .map_err(|error| NetworkError::ProtocolError(error.to_string()))
}

fn host_header(spec: &RequestSpec) -> String {
    let key = host_key(spec.url.host());
    let default_port = if spec.url.is_https() { 443 } else { 80 };
    if spec.url.port() == default_port {
        key
    } else {
        format!("{key}:{}", spec.url.port())
    }
}
