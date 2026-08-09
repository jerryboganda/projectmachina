import assert from "node:assert/strict";
import { createFixtureServer } from "./fixture-server.mjs";

const CLIENTS = ["http", "grpc", "typescript", "python"];

export class CompatibilityControlPlane {
  constructor(snapshot = undefined) {
    this.sessions = new Map(snapshot?.sessions ?? []);
    this.traces = new Map(snapshot?.traces ?? []);
    this.events = new Map(snapshot?.events ?? []);
    this.workerState = snapshot?.workerState ?? "ready";
    this.disconnectOnce = true;
  }

  async execute(command, client) {
    const trace = this.traceFor(command.session_id);
    this.recordTrace(trace, "api.request.v1", `${client}:accepted`, command);
    if (command.cancelled === true) {
      return this.failure(command, "COMMAND_CANCELLED", "command cancelled", trace);
    }
    if (this.workerState !== "ready") {
      return this.failure(command, "WORKER_LOST", "worker is unavailable", trace);
    }
    if (!["session.create.v1", "navigation.goto.v1", "dom.semantic_query.v1", "session.close.v1"].includes(command.kind)) {
      return this.failure(
        command,
        "UNSUPPORTED_CAPABILITY",
        `unsupported command: ${command.kind}`,
        trace
      );
    }

    if (command.kind === "session.create.v1") {
      const current = this.sessions.get(command.session_id);
      if (current?.state === "ready") {
        return this.success(command, "session ready", trace);
      }
      this.sessions.set(command.session_id, { state: "ready", url: null });
      this.publish(command.session_id, "session.lifecycle.v1", { state: "ready" });
      return this.success(command, "session ready", trace);
    }

    const session = this.sessions.get(command.session_id);
    if (!session || session.state !== "ready") {
      return this.failure(command, "SESSION_NOT_READY", "session is not ready", trace);
    }
    if (command.kind === "navigation.goto.v1") {
      const response = await fetch(command.payload.url);
      if (!response.ok) {
        return this.failure(command, "NAVIGATION_FAILED", "fixture navigation failed", trace);
      }
      session.url = command.payload.url;
      this.publish(command.session_id, "navigation.lifecycle.v1", {
        url: command.payload.url,
        status: response.status
      });
      return this.success(command, "navigation verified", trace);
    }
    if (command.kind === "dom.semantic_query.v1") {
      this.publish(command.session_id, "dom.revision.v1", { query: command.payload.query });
      return this.success(command, "Navigation fixture", trace);
    }
    session.state = "closed";
    this.publish(command.session_id, "session.lifecycle.v1", { state: "closed" });
    return this.success(command, "session closed", trace);
  }

  async *subscribe(sessionId, afterSequence) {
    if (this.disconnectOnce) {
      this.disconnectOnce = false;
      throw new Error("simulated client disconnect");
    }
    const events = this.events.get(sessionId) ?? [];
    for (const event of events) {
      if (event.sequence > afterSequence) {
        yield event;
      }
    }
  }

  crashWorker() {
    this.workerState = "lost";
  }

  restartWorker() {
    this.workerState = "ready";
  }

  restartControlPlane() {
    return new CompatibilityControlPlane({
      sessions: [...this.sessions.entries()],
      traces: [...this.traces.entries()],
      events: [...this.events.entries()],
      workerState: this.workerState
    });
  }

  traceFor(sessionId) {
    if (!this.traces.has(sessionId)) {
      this.traces.set(sessionId, []);
    }
    return this.traces.get(sessionId);
  }

  recordTrace(trace, eventType, payload, command) {
    trace.push({
      sequence: trace.length + 1,
      event_type: eventType,
      payload,
      command_id: command.command_id
    });
  }

  publish(sessionId, eventType, payload) {
    const events = this.events.get(sessionId) ?? [];
    events.push({
      event_id: `${sessionId}:${events.length + 1}`,
      sequence: events.length + 1,
      event_type: eventType,
      session_id: sessionId,
      payload: JSON.stringify(payload)
    });
    this.events.set(sessionId, events);
  }

  success(command, result, trace) {
    this.recordTrace(trace, "worker.outcome.v1", "verified", command);
    return {
      command_id: command.command_id,
      attempt: 1,
      status: "succeeded",
      result,
      execution: {
        requested_engine_policy: "prefer-compatible",
        engine: "chromium",
        engine_version: "fixture-chromium",
        capability_snapshot: "capability.v0",
        fallback_used: false
      },
      trace_ref: `trace-${command.session_id}`
    };
  }

  failure(command, code, message, trace) {
    this.recordTrace(trace, "command.error.v1", code, command);
    return {
      command_id: command.command_id,
      attempt: 1,
      status: code === "COMMAND_CANCELLED" ? "cancelled" : "failed",
      error: {
        code,
        category: "compatibility-smoke",
        message,
        retryable: code === "WORKER_LOST",
        command_id: command.command_id,
        correlation_id: command.metadata.correlation_id,
        details: {},
        documentation_ref: `errors/${code}`
      },
      execution: {
        requested_engine_policy: "prefer-compatible",
        engine: "chromium",
        engine_version: "fixture-chromium",
        capability_snapshot: "capability.v0",
        fallback_used: false
      },
      trace_ref: `trace-${command.session_id}`
    };
  }
}

function command(sessionId, kind, capability, payload, client, options = {}) {
  return {
    command_id: `${client}-${kind}-${sessionId}`,
    session_id: sessionId,
    kind,
    payload,
    deadline_ms: 5_000,
    required_capabilities: [capability],
    metadata: {
      correlation_id: `${client}-correlation-${sessionId}`,
      client
    },
    ...options
  };
}

async function collectEvents(controlPlane, sessionId) {
  let after = 0;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const received = [];
      for await (const event of controlPlane.subscribe(sessionId, after)) {
        received.push(event);
        after = event.sequence;
      }
      if (received.length > 0) {
        return received;
      }
    } catch (error) {
      if (attempt === 2) {
        throw error;
      }
    }
  }
  throw new Error("event reconnect did not receive a batch");
}

export async function runM1CompatibilitySmoke() {
  const fixture = createFixtureServer();
  const address = await fixture.start();
  try {
    const fixtureUrl = `${address.protocol}://${address.host}:${address.port}/navigation`;
    const form = await fetch(`${address.protocol}://${address.host}:${address.port}/form`, {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: "name=compatibility"
    });
    assert.equal(form.status, 200);
    assert.deepEqual(await form.json(), {
      accepted: true,
      body: "name=compatibility"
    });

    const journeys = [];
    for (const client of CLIENTS) {
      const controlPlane = new CompatibilityControlPlane();
      const sessionId = `${client}-session`;
      const transport = {
        execute: (request) => controlPlane.execute(request, client),
        subscribe: (id, after) => controlPlane.subscribe(id, after)
      };
      const created = await transport.execute(
        command(sessionId, "session.create.v1", "session.create.v1", {
          engine_policy: "prefer-compatible",
          fidelity_profile: "agent"
        }, client)
      );
      const navigated = await transport.execute(
        command(sessionId, "navigation.goto.v1", "navigation.goto.v1", {
          url: fixtureUrl,
          wait_until: "load"
        }, client)
      );
      const extracted = await transport.execute(
        command(sessionId, "dom.semantic_query.v1", "dom.semantic_query.v1", {
          query: "main h1"
        }, client)
      );
      const closed = await transport.execute(
        command(sessionId, "session.close.v1", "session.close.v1", {
          reason: "smoke"
        }, client)
      );
      assert.equal(created.status, "succeeded");
      assert.equal(navigated.status, "succeeded");
      assert.equal(extracted.result, "Navigation fixture");
      assert.equal(closed.result, "session closed");
      const events = await collectEvents(controlPlane, sessionId);
      const trace = controlPlane.traces.get(sessionId);
      assert.ok(trace.some((event) => event.event_type === "api.request.v1"));
      assert.ok(trace.some((event) => event.event_type === "worker.outcome.v1"));
      journeys.push({
        surface: client,
        session_id: sessionId,
        trace_ref: extracted.trace_ref,
        event_count: events.length
      });
    }

    const failures = new CompatibilityControlPlane();
    const failureCommand = command(
      "failure-session",
      "session.create.v1",
      "session.create.v1",
      { engine_policy: "prefer-compatible", fidelity_profile: "agent" },
      "failure"
    );
    await failures.execute(failureCommand, "failure");
    const cancelled = await failures.execute(
      command(
        "failure-session",
        "navigation.goto.v1",
        "navigation.goto.v1",
        { url: fixtureUrl, wait_until: "load" },
        "failure",
        { cancelled: true }
      ),
      "failure"
    );
    const unsupported = await failures.execute(
      command(
        "failure-session",
        "interaction.click.v1",
        "interaction.click.v1",
        { selector: "#missing" },
        "failure"
      ),
      "failure"
    );
    assert.equal(cancelled.error.code, "COMMAND_CANCELLED");
    assert.equal(unsupported.error.code, "UNSUPPORTED_CAPABILITY");

    failures.crashWorker();
    const lost = await failures.execute(
      command(
        "failure-session",
        "navigation.goto.v1",
        "navigation.goto.v1",
        { url: fixtureUrl, wait_until: "load" },
        "failure"
      ),
      "failure"
    );
    assert.equal(lost.error.code, "WORKER_LOST");
    failures.restartWorker();
    const reconciled = failures.restartControlPlane();
    assert.equal(reconciled.workerState, "ready");
    assert.equal(reconciled.sessions.get("failure-session").state, "ready");

    return {
      status: "injected-command-core-smoke-passed",
      surface_mode: "labels only; not live HTTP/gRPC/SDK clients",
      fixture: fixture.manifest.version,
      journeys,
      fixture_form_sanity: "passed",
      explicit_failures: ["COMMAND_CANCELLED", "UNSUPPORTED_CAPABILITY", "WORKER_LOST"],
      restart_reconciliation: "passed",
      runtime_claim: "injected Chromium contract only; real process/container runtime unavailable under owner waiver"
    };
  } finally {
    await fixture.stop();
  }
}

const invokedUrl = process.argv[1]
  ? new URL(`file:///${process.argv[1].replaceAll("\\", "/")}`).href
  : "";
if (import.meta.url === invokedUrl) {
  console.log(JSON.stringify(await runM1CompatibilitySmoke(), null, 2));
}
