# Chromium worker pool boundary

The worker-pool crate owns explicit lifecycle, lease, generation, and
isolation-tier state. This service directory is the integration boundary for
the external Chromium runtime. A runtime provider must report prewarm failure
as `RuntimeUnavailable`; the pool never converts an unavailable browser into a
ready worker.

`machina-chromium-adapter` is the canonical command-bus adapter. It exposes
engine/build/capability metadata and accepts only an injected transport, so
protocol adapters cannot bypass the command bus or silently emulate Chromium.
