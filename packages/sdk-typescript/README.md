# Project Machina TypeScript SDK (alpha)

The SDK is an async, transport-neutral facade over generated canonical
contracts. It provides typed `MachinaError` codes, fetch deadlines and
`AbortSignal` cancellation, session/page helpers, reconnecting event streams,
and idempotent close cleanup.

```ts
import { HttpTransport, MachinaClient } from "@machina/sdk-typescript";

const client = new MachinaClient(new HttpTransport("http://127.0.0.1:8080"));
const session = await client.createSession();
await session.navigate("https://fixture.local/");
const result = await session.page().extract("main article");
await session.close();
```

The HTTP transport expects `POST /v1/commands` and SSE
`/v1/sessions/{session_id}/events?after_sequence=N`. Inject `CommandTransport`
for clean tests or alternate protocol deployments.
