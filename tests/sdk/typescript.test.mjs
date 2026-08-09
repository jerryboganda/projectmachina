import assert from "node:assert/strict";
import test from "node:test";
import { MachinaClient } from "../../packages/sdk-typescript/dist/index.js";

class FakeTransport {
  commands = [];
  attempts = 0;

  async execute(command) {
    this.commands.push(command);
    return {
      command_id: command.command_id,
      attempt: 1,
      status: "succeeded",
      result: "ok",
      execution: {
        requested_engine_policy: "prefer-compatible",
        engine: "chromium",
        engine_version: "fixture",
        capability_snapshot: "capability.v0",
        fallback_used: false
      },
      trace_ref: "trace-1"
    };
  }

  async *subscribe(sessionId, afterSequence) {
    this.attempts += 1;
    if (this.attempts === 1) {
      throw new Error("fixture disconnect");
    }
    yield {
      sequence: afterSequence + 1,
      event_type: "session.lifecycle.v1",
      payload: "{}",
      session_id: sessionId
    };
  }
}

test("TypeScript facade completes lifecycle and reconnects events", async () => {
  const transport = new FakeTransport();
  const session = await new MachinaClient(transport).createSession();
  await session.navigate("https://fixture.local/");
  await session.page().extract("main");
  await session.close();
  assert.deepEqual(
    transport.commands.map((command) => command.kind),
    [
      "session.create.v1",
      "navigation.goto.v1",
      "dom.semantic_query.v1",
      "session.close.v1"
    ]
  );
  const events = [];
  const abort = new AbortController();
  for await (const event of session.events({ reconnectDelayMs: 0, signal: abort.signal })) {
    events.push(event);
    abort.abort();
  }
  assert.equal(events[0].sequence, 1);
  assert.equal(transport.attempts, 2);
});
