# Project Machina Python SDK (alpha)

The SDK is an async, transport-neutral facade over the canonical command
model. It provides typed `MachinaError` codes, deadlines/cancellation,
session/page helpers, reconnecting event streams, and idempotent close cleanup.

```python
from machina_sdk import HttpTransport, MachinaClient

client = MachinaClient(HttpTransport("http://127.0.0.1:8080"))
session = await client.create_session()
await session.navigate("https://fixture.local/")
result = await session.page().extract("main article")
await session.close()
```

The stdlib transport expects `POST /v1/commands` and SSE
`/v1/sessions/{session_id}/events?after_sequence=N`. A custom `AsyncTransport`
can be injected for tests, self-hosted deployments, or alternate protocols.
Object/page content is returned only by explicit command results; the SDK does
not log credentials or page payloads.
