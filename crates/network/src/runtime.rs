//! The crate's privately-owned async runtime and the deadline/cancellation
//! guard combinator. The rest of the engine (`command-bus`, `session`) is
//! synchronous today (M2-T08's event loop, which will drive this crate
//! asynchronously, depends on M2-T02, not the reverse) -- `network` is
//! self-contained: it owns a small Tokio runtime and exposes a
//! sync-callable public API (`NetworkClient::fetch`), built on internal
//! `async`/`await` machinery that an async event loop can reuse later
//! without a redesign (the body type already implements `http_body::Body`).

use std::future::Future;
use std::time::Duration;

use machina_command_bus::CommandContext;

use crate::error::NetworkError;

/// A dedicated, per-`NetworkClient` Tokio runtime (design section 1: "small
/// dedicated runtime, not workspace-wide").
pub struct NetworkRuntime {
    runtime: tokio::runtime::Runtime,
}

impl NetworkRuntime {
    pub fn new() -> Result<Self, NetworkError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|error| NetworkError::Io(error.to_string()))?;
        Ok(Self { runtime })
    }

    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

/// How often the guard polls `ctx.cancellation` while waiting on `fut`.
/// `CancellationToken` (owned by `machina-command-bus`) has no waker today,
/// so this interval poll is the documented fallback the M2-T02 design calls
/// for (section 4) rather than a breaking/additive change to
/// `machina-command-bus` -- 25ms keeps cancellation-to-teardown latency low
/// without meaningfully increasing CPU use for the lifetime of one request.
const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Race `fut` against `ctx.deadline` and `ctx.cancellation`. Used around
/// every I/O phase that can block indefinitely: connect, TLS handshake,
/// header read, and every individual body chunk read -- not just the
/// initial request (defends against slow-loris after headers arrive).
pub async fn guarded<T, F>(ctx: &CommandContext, fut: F) -> Result<T, NetworkError>
where
    F: Future<Output = Result<T, NetworkError>>,
{
    // `tokio::time::sleep_until` with an already-past deadline resolves on
    // its very first poll, and `tokio::time::interval` fires immediately on
    // its first `tick()` -- so an already-expired deadline or an
    // already-cancelled token is caught on the first iteration of the loop
    // below without a separate fast-path check (which would otherwise
    // require depending on `machina-command-model` just to name
    // `CanonicalErrorCode`, a dependency this crate intentionally avoids).
    let deadline = tokio::time::Instant::from_std(ctx.deadline);
    tokio::pin!(fut);
    let mut poll_interval = tokio::time::interval(CANCELLATION_POLL_INTERVAL);
    loop {
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(deadline) => return Err(NetworkError::DeadlineExceeded),
            _ = poll_interval.tick() => {
                if ctx.cancellation.is_cancelled() {
                    return Err(NetworkError::Cancelled);
                }
            }
            result = &mut fut => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::guarded;
    use crate::error::NetworkError;
    use machina_command_bus::CommandContext;
    use std::time::Duration;

    #[tokio::test]
    async fn guard_returns_deadline_exceeded_when_future_never_completes() {
        let ctx = CommandContext::with_timeout("correlation-1", Duration::from_millis(50));
        let result = guarded(&ctx, async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<(), NetworkError>(())
        })
        .await;
        assert_eq!(result, Err(NetworkError::DeadlineExceeded));
    }

    #[tokio::test]
    async fn guard_returns_cancelled_when_token_is_cancelled_mid_flight() {
        let ctx = CommandContext::with_timeout("correlation-1", Duration::from_secs(5));
        let token = ctx.cancellation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(40)).await;
            token.cancel();
        });
        let result = guarded(&ctx, async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<(), NetworkError>(())
        })
        .await;
        assert_eq!(result, Err(NetworkError::Cancelled));
    }

    #[tokio::test]
    async fn guard_passes_through_the_inner_result_when_it_finishes_first() {
        let ctx = CommandContext::with_timeout("correlation-1", Duration::from_secs(5));
        let result = guarded(&ctx, async { Ok::<u32, NetworkError>(7) }).await;
        assert_eq!(result, Ok(7));
    }
}
