//! Streaming response body: deadline/cancellation-guarded raw reads,
//! streaming decompression, and incremental budget/ratio metering. There is
//! no "read whole body into `Vec`" path in the primary API -- only the
//! explicitly-named, budget-capped `read_to_end_bounded` helper for known
//! -small payloads, which still goes through the same per-chunk checks.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use http_body::{Body, Frame};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::BodyExt;
use machina_command_bus::CommandContext;
use tokio::io::BufReader;
use tokio_util::io::{ReaderStream, StreamReader};

use crate::budget::RequestBudget;
use crate::error::NetworkError;

#[derive(Clone, Copy, Debug)]
pub struct ResponseLimits {
    /// Absolute ceiling on decompressed bytes for one response body.
    pub max_body_bytes: u64,
    /// Ceiling on decompressed_bytes / max(compressed_bytes, 1); a
    /// zip-bomb control independent of the absolute ceiling.
    pub max_decompression_ratio: u64,
    pub max_header_bytes: usize,
    pub max_header_count: usize,
}

impl Default for ResponseLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: 256 * 1024 * 1024,
            max_decompression_ratio: 100,
            max_header_bytes: 64 * 1024,
            max_header_count: 128,
        }
    }
}

/// Wraps `hyper::body::Incoming` so every individual frame read is raced
/// against `ctx.deadline`/`ctx.cancellation`/an idle-read timeout at the
/// true poll layer (each is a `tokio::time::Sleep`/`Interval` that
/// registers its own waker, so a completely idle/slow-trickled connection
/// is still woken and aborted -- this is not a best-effort check that only
/// fires when something else happens to re-poll the future). The idle
/// timer is distinct from the absolute deadline: it re-arms after every
/// frame received, so it bounds the gap between chunks (and the initial
/// time-to-first-byte) independently of the request's total time budget --
/// the security review's "distinct connect / handshake / time-to-first-byte
/// / idle-read timeouts, not one overall deadline" requirement.
struct DeadlineGuardedBody {
    inner: hyper::body::Incoming,
    ctx: CommandContext,
    sleep: Pin<Box<tokio::time::Sleep>>,
    cancel_check: tokio::time::Interval,
    idle_timeout: std::time::Duration,
    idle_sleep: Pin<Box<tokio::time::Sleep>>,
}

impl DeadlineGuardedBody {
    fn new(
        inner: hyper::body::Incoming,
        ctx: CommandContext,
        idle_timeout: std::time::Duration,
    ) -> Self {
        let sleep = Box::pin(tokio::time::sleep_until(tokio::time::Instant::from_std(
            ctx.deadline,
        )));
        let mut cancel_check = tokio::time::interval(std::time::Duration::from_millis(25));
        cancel_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let idle_sleep = Box::pin(tokio::time::sleep(idle_timeout));
        Self {
            inner,
            ctx,
            sleep,
            cancel_check,
            idle_timeout,
            idle_sleep,
        }
    }
}

impl Body for DeadlineGuardedBody {
    type Data = Bytes;
    type Error = NetworkError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, NetworkError>>> {
        if self.sleep.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Some(Err(NetworkError::DeadlineExceeded)));
        }
        if self.cancel_check.poll_tick(cx).is_ready() && self.ctx.cancellation.is_cancelled() {
            return Poll::Ready(Some(Err(NetworkError::Cancelled)));
        }
        if self.idle_sleep.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Some(Err(NetworkError::IdleReadTimeout)));
        }
        match Pin::new(&mut self.inner).poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                let idle_timeout = self.idle_timeout;
                self.idle_sleep.set(tokio::time::sleep(idle_timeout));
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(error))) => {
                Poll::Ready(Some(Err(NetworkError::ProtocolError(error.to_string()))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Supported `Content-Encoding` values. Anything else (including a stacked
/// list like `gzip, br`) fails closed at construction time rather than
/// being passed through as uncompressed or guessed at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Encoding {
    Identity,
    Gzip,
    Deflate,
    Brotli,
}

fn parse_content_encoding(value: Option<&str>) -> Result<Encoding, NetworkError> {
    let Some(value) = value else {
        return Ok(Encoding::Identity);
    };
    let tokens: Vec<&str> = value
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    match tokens.as_slice() {
        [] => Ok(Encoding::Identity),
        ["identity"] => Ok(Encoding::Identity),
        ["gzip"] | ["x-gzip"] => Ok(Encoding::Gzip),
        ["deflate"] => Ok(Encoding::Deflate),
        ["br"] => Ok(Encoding::Brotli),
        [single] => Err(NetworkError::DecompressionFailed(format!(
            "unsupported content-encoding: {single}"
        ))),
        _ => Err(NetworkError::DecompressionFailed(
            "stacked content-encoding values are not supported".to_owned(),
        )),
    }
}

fn io_error(error: NetworkError) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

fn map_io_result(result: std::io::Result<Bytes>) -> Result<Bytes, NetworkError> {
    result.map_err(|error| NetworkError::DecompressionFailed(error.to_string()))
}

/// Build the fully-composed body pipeline: guarded raw reads -> optional
/// streaming decompression -> incremental absolute/ratio/budget metering.
/// Nothing here buffers the whole body; every stage is a `Stream` that
/// yields chunks as they become available.
fn build_pipeline(
    incoming: hyper::body::Incoming,
    content_encoding: Option<&str>,
    limits: ResponseLimits,
    budget: Arc<dyn RequestBudget>,
    ctx: CommandContext,
    idle_timeout: std::time::Duration,
) -> Result<UnsyncBoxBody<Bytes, NetworkError>, NetworkError> {
    let encoding = parse_content_encoding(content_encoding)?;
    let guarded = DeadlineGuardedBody::new(incoming, ctx, idle_timeout);
    let raw_stream = guarded.into_data_stream();

    let compressed_counter = Arc::new(AtomicU64::new(0));
    let counted_stream = {
        let counter = compressed_counter.clone();
        raw_stream.map(move |item| {
            if let Ok(bytes) = &item {
                counter.fetch_add(bytes.len() as u64, Ordering::Relaxed);
            }
            item
        })
    };

    let decoded_stream: Pin<Box<dyn Stream<Item = Result<Bytes, NetworkError>> + Send>> =
        match encoding {
            Encoding::Identity => Box::pin(counted_stream),
            Encoding::Gzip => Box::pin(
                ReaderStream::new(async_compression::tokio::bufread::GzipDecoder::new(
                    BufReader::new(StreamReader::new(
                        counted_stream.map(|item| item.map_err(io_error)),
                    )),
                ))
                .map(map_io_result),
            ),
            // HTTP's `deflate` Content-Encoding is, despite the name,
            // conventionally the zlib-wrapped format (RFC 1950) in practice
            // (Node's `zlib.deflateSync`, most browsers, and most servers), not
            // raw DEFLATE (RFC 1951) -- `ZlibDecoder` is the correct decoder.
            Encoding::Deflate => Box::pin(
                ReaderStream::new(async_compression::tokio::bufread::ZlibDecoder::new(
                    BufReader::new(StreamReader::new(
                        counted_stream.map(|item| item.map_err(io_error)),
                    )),
                ))
                .map(map_io_result),
            ),
            Encoding::Brotli => Box::pin(
                ReaderStream::new(async_compression::tokio::bufread::BrotliDecoder::new(
                    BufReader::new(StreamReader::new(
                        counted_stream.map(|item| item.map_err(io_error)),
                    )),
                ))
                .map(map_io_result),
            ),
        };

    let metered = meter_stream(decoded_stream, limits, budget, compressed_counter);
    let body = http_body_util::StreamBody::new(metered.map(|result| result.map(Frame::data)));
    Ok(BodyExt::boxed_unsync(body))
}

fn meter_stream(
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, NetworkError>> + Send>>,
    limits: ResponseLimits,
    budget: Arc<dyn RequestBudget>,
    compressed_counter: Arc<AtomicU64>,
) -> impl Stream<Item = Result<Bytes, NetworkError>> {
    async_stream::try_stream! {
        tokio::pin!(stream);
        let mut decompressed_total: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            decompressed_total = decompressed_total.saturating_add(chunk.len() as u64);
            if decompressed_total > limits.max_body_bytes {
                Err(NetworkError::BodyTooLarge { limit: limits.max_body_bytes })?;
            }
            let compressed = compressed_counter.load(Ordering::Relaxed).max(1);
            if decompressed_total / compressed > limits.max_decompression_ratio {
                Err(NetworkError::DecompressionFailed(format!(
                    "decompression ratio exceeded configured ceiling of {}x",
                    limits.max_decompression_ratio
                )))?;
            }
            budget.account_bytes(chunk.len() as u64)?;
            yield chunk;
        }
    }
}

/// The public streaming response body. Implements `http_body::Body`
/// directly (forward-compatible with a future async event loop driving it
/// without a second body abstraction) and additionally exposes a
/// blocking-facade helper for today's synchronous callers.
pub struct ResponseBody {
    inner: UnsyncBoxBody<Bytes, NetworkError>,
}

impl ResponseBody {
    pub fn from_incoming(
        incoming: hyper::body::Incoming,
        content_encoding: Option<&str>,
        limits: ResponseLimits,
        budget: Arc<dyn RequestBudget>,
        ctx: CommandContext,
        idle_timeout: std::time::Duration,
    ) -> Result<Self, NetworkError> {
        Ok(Self {
            inner: build_pipeline(
                incoming,
                content_encoding,
                limits,
                budget,
                ctx,
                idle_timeout,
            )?,
        })
    }

    pub fn empty() -> Self {
        Self {
            inner: BodyExt::boxed_unsync(
                http_body_util::Empty::new()
                    .map_err(|error: std::convert::Infallible| match error {}),
            ),
        }
    }

    /// Read and return the next data chunk, blocking the calling thread on
    /// `runtime` until it is available (or an error/deadline/cancellation
    /// occurs). Returns `Ok(None)` at end of body. This is the primary
    /// streaming read path -- it never buffers more than one chunk.
    pub fn next_chunk_blocking(
        &mut self,
        runtime: &tokio::runtime::Handle,
    ) -> Result<Option<Bytes>, NetworkError> {
        runtime.block_on(async {
            loop {
                match std::future::poll_fn(|cx| Pin::new(&mut self.inner).poll_frame(cx)).await {
                    None => return Ok(None),
                    Some(Err(error)) => return Err(error),
                    Some(Ok(frame)) => {
                        if let Ok(data) = frame.into_data() {
                            return Ok(Some(data));
                        }
                        // Trailers frame: skip, keep reading for the end.
                    }
                }
            }
        })
    }

    /// Read the whole body up to `max_bytes`, still going through the same
    /// per-chunk budget/limit checks as streaming reads, and aborting with
    /// `BodyTooLarge` rather than silently truncating if the body is
    /// larger. The only place in the public API that returns a fully
    /// buffered `Vec<u8>` -- intended for known-small payloads only.
    pub fn read_to_end_bounded(
        &mut self,
        max_bytes: u64,
        runtime: &tokio::runtime::Handle,
    ) -> Result<Vec<u8>, NetworkError> {
        let mut buffer = Vec::new();
        while let Some(chunk) = self.next_chunk_blocking(runtime)? {
            if buffer.len() as u64 + chunk.len() as u64 > max_bytes {
                return Err(NetworkError::BodyTooLarge { limit: max_bytes });
            }
            buffer.extend_from_slice(&chunk);
        }
        Ok(buffer)
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = NetworkError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, NetworkError>>> {
        Pin::new(&mut self.inner).poll_frame(cx)
    }
}
