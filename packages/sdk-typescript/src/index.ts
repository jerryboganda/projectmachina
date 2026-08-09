import type {
  CanonicalError as CanonicalErrorType,
  CanonicalErrorCode as CanonicalErrorCodeType,
  CommandKind as CommandKindType,
  CommandEnvelope as CommandEnvelopeType,
  CommandOutcome as CommandOutcomeType,
  ClickPayload,
  EngineExecution as EngineExecutionType,
  EngineKind as EngineKindType,
  EnginePolicy as EnginePolicyType,
  FidelityProfile as FidelityProfileType,
  NavigationGotoPayload,
  OutcomeStatus as OutcomeStatusType,
  SemanticQueryPayload,
  SessionClosePayload,
  SessionCreatePayload,
  WaitUntil as WaitUntilType
} from "@machina/contracts-ts";

export type CanonicalError = CanonicalErrorType;
export type CanonicalErrorCode = CanonicalErrorCodeType;
export type CommandEnvelope = CommandEnvelopeType;
export type CommandOutcome = CommandOutcomeType;
export type EngineExecution = EngineExecutionType;
export type EngineKind = EngineKindType;
export type EnginePolicy = EnginePolicyType;
export type FidelityProfile = FidelityProfileType;
export type OutcomeStatus = OutcomeStatusType;
export type WaitUntil = WaitUntilType;

function wireEnum<T extends string>(value: string): T {
  return value as T;
}

export const CanonicalErrorCode = {
  invalidArgument: wireEnum<CanonicalErrorCodeType>("INVALID_ARGUMENT"),
  unauthenticated: wireEnum<CanonicalErrorCodeType>("UNAUTHENTICATED"),
  permissionDenied: wireEnum<CanonicalErrorCodeType>("PERMISSION_DENIED"),
  policyDenied: wireEnum<CanonicalErrorCodeType>("POLICY_DENIED"),
  quotaExceeded: wireEnum<CanonicalErrorCodeType>("QUOTA_EXCEEDED"),
  rateLimited: wireEnum<CanonicalErrorCodeType>("RATE_LIMITED"),
  sessionNotReady: wireEnum<CanonicalErrorCodeType>("SESSION_NOT_READY"),
  sessionClosed: wireEnum<CanonicalErrorCodeType>("SESSION_CLOSED"),
  sessionExpired: wireEnum<CanonicalErrorCodeType>("SESSION_EXPIRED"),
  capacityUnavailable: wireEnum<CanonicalErrorCodeType>("CAPACITY_UNAVAILABLE"),
  workerLost: wireEnum<CanonicalErrorCodeType>("WORKER_LOST"),
  commandCancelled: wireEnum<CanonicalErrorCodeType>("COMMAND_CANCELLED"),
  deadlineExceeded: wireEnum<CanonicalErrorCodeType>("DEADLINE_EXCEEDED"),
  unsupportedCapability: wireEnum<CanonicalErrorCodeType>("UNSUPPORTED_CAPABILITY"),
  capabilityDisabled: wireEnum<CanonicalErrorCodeType>("CAPABILITY_DISABLED"),
  rendererRequired: wireEnum<CanonicalErrorCodeType>("RENDERER_REQUIRED"),
  fallbackProhibited: wireEnum<CanonicalErrorCodeType>("FALLBACK_PROHIBITED"),
  migrationFailed: wireEnum<CanonicalErrorCodeType>("MIGRATION_FAILED"),
  stateTransferPartial: wireEnum<CanonicalErrorCodeType>("STATE_TRANSFER_PARTIAL"),
  invalidUrl: wireEnum<CanonicalErrorCodeType>("INVALID_URL"),
  networkPolicyBlocked: wireEnum<CanonicalErrorCodeType>("NETWORK_POLICY_BLOCKED"),
  navigationFailed: wireEnum<CanonicalErrorCodeType>("NAVIGATION_FAILED"),
  selectorInvalid: wireEnum<CanonicalErrorCodeType>("SELECTOR_INVALID"),
  elementNotFound: wireEnum<CanonicalErrorCodeType>("ELEMENT_NOT_FOUND"),
  elementAmbiguous: wireEnum<CanonicalErrorCodeType>("ELEMENT_AMBIGUOUS"),
  elementNotInteractable: wireEnum<CanonicalErrorCodeType>("ELEMENT_NOT_INTERACTABLE"),
  actionPostconditionFailed: wireEnum<CanonicalErrorCodeType>("ACTION_POSTCONDITION_FAILED"),
  workflowInvalid: wireEnum<CanonicalErrorCodeType>("WORKFLOW_INVALID"),
  approvalRequired: wireEnum<CanonicalErrorCodeType>("APPROVAL_REQUIRED"),
  secretUnavailable: wireEnum<CanonicalErrorCodeType>("SECRET_UNAVAILABLE")
} as const;

export const CommandKind = {
  sessionCreateV1: wireEnum<CommandKindType.sessionCreateV1>("session.create.v1"),
  navigationGotoV1: wireEnum<CommandKindType.navigationGotoV1>("navigation.goto.v1"),
  domSemanticQueryV1: wireEnum<CommandKindType.domSemanticQueryV1>("dom.semantic_query.v1"),
  interactionClickV1: wireEnum<CommandKindType.interactionClickV1>("interaction.click.v1"),
  sessionCloseV1: wireEnum<CommandKindType.sessionCloseV1>("session.close.v1")
} as const;

export const EngineKind = {
  chromium: wireEnum<EngineKindType>("chromium"),
  native: wireEnum<EngineKindType>("native")
} as const;

export const EnginePolicy = {
  nativeOnly: wireEnum<EnginePolicyType>("native-only"),
  preferNative: wireEnum<EnginePolicyType>("prefer-native"),
  preferCompatible: wireEnum<EnginePolicyType>("prefer-compatible"),
  chromiumOnly: wireEnum<EnginePolicyType>("chromium-only")
} as const;

export const FidelityProfile = {
  extract: wireEnum<FidelityProfileType>("extract"),
  agent: wireEnum<FidelityProfileType>("agent"),
  test: wireEnum<FidelityProfileType>("test"),
  visual: wireEnum<FidelityProfileType>("visual"),
  custom: wireEnum<FidelityProfileType>("custom")
} as const;

export const OutcomeStatus = {
  succeeded: wireEnum<OutcomeStatusType>("succeeded"),
  failed: wireEnum<OutcomeStatusType>("failed"),
  cancelled: wireEnum<OutcomeStatusType>("cancelled"),
  deadlineExceeded: wireEnum<OutcomeStatusType>("deadline_exceeded")
} as const;

export const WaitUntil = {
  commit: wireEnum<WaitUntilType>("commit"),
  domcontentloaded: wireEnum<WaitUntilType>("domcontentloaded"),
  load: wireEnum<WaitUntilType>("load"),
  networkidle: wireEnum<WaitUntilType>("networkidle")
} as const;

export interface SessionEvent {
  event_id?: string;
  sequence: number;
  event_type: string;
  session_id?: string;
  payload: string;
  correlation_id?: string;
  timestamp?: string;
}

export interface ExecuteOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
}

export interface SubscribeOptions {
  signal?: AbortSignal;
  afterSequence?: number;
  maxReconnectAttempts?: number;
  reconnectDelayMs?: number;
}

export interface CommandTransport {
  execute(command: CommandEnvelope, options?: ExecuteOptions): Promise<CommandOutcome>;
  subscribe(
    sessionId: string,
    afterSequence: number,
    signal?: AbortSignal
  ): AsyncIterable<SessionEvent>;
}

export class MachinaError extends Error {
  readonly code: CanonicalErrorCode;
  readonly canonical: CanonicalError;
  readonly status?: number;

  constructor(canonical: CanonicalError, status?: number) {
    super(canonical.message);
    this.name = "MachinaError";
    this.code = canonical.code;
    this.canonical = canonical;
    this.status = status;
  }
}

export class HttpTransport implements CommandTransport {
  private readonly fetchImpl: typeof fetch;
  private readonly headers: HeadersInit;

  constructor(
    private readonly baseUrl: string,
    fetchImpl: typeof fetch = globalThis.fetch,
    headers: HeadersInit = {}
  ) {
    if (typeof fetchImpl !== "function") {
      throw new Error("a fetch implementation is required");
    }
    this.fetchImpl = fetchImpl;
    this.headers = headers;
  }

  async execute(command: CommandEnvelope, options: ExecuteOptions = {}): Promise<CommandOutcome> {
    const response = await this.fetchWithDeadline(
      `${this.baseUrl.replace(/\/$/, "")}/v1/commands`,
      {
        method: "POST",
        headers: {
          ...this.headers,
          "content-type": "application/json",
          accept: "application/json"
        },
        body: JSON.stringify(command)
      },
      options
    );
    const body = await readJson(response);
    if (!response.ok) {
      throw new MachinaError(parseCanonicalError(recordField(body, "error")), response.status);
    }
    return parseOutcome(body);
  }

  async *subscribe(
    sessionId: string,
    afterSequence: number,
    signal?: AbortSignal
  ): AsyncIterable<SessionEvent> {
    const response = await this.fetchImpl(
      `${this.baseUrl.replace(/\/$/, "")}/v1/sessions/${encodeURIComponent(
        sessionId
      )}/events?after_sequence=${afterSequence}`,
      {
        headers: { ...this.headers, accept: "text/event-stream" },
        signal
      }
    );
    if (!response.ok) {
      const body = await readJson(response);
      throw new MachinaError(parseCanonicalError(recordField(body, "error")), response.status);
    }
    if (!response.body) {
      throw new Error("event stream response has no body");
    }
    yield* parseEventStream(response.body);
  }

  private async fetchWithDeadline(
    input: RequestInfo | URL,
    init: RequestInit,
    options: ExecuteOptions
  ): Promise<Response> {
    const controller = new AbortController();
    if (options.signal?.aborted) {
      throw new MachinaError(
        syntheticError(CanonicalErrorCode.commandCancelled, "command cancelled")
      );
    }
    let timedOut = false;
    const timer =
      options.timeoutMs === undefined
        ? undefined
        : setTimeout(() => {
            timedOut = true;
            controller.abort();
          }, options.timeoutMs);
    const abort = () => controller.abort();
    options.signal?.addEventListener("abort", abort, { once: true });
    try {
      return await this.fetchImpl(input, { ...init, signal: controller.signal });
    } catch (error: unknown) {
      if (isAbortError(error)) {
        throw new MachinaError(
          syntheticError(
            timedOut
              ? CanonicalErrorCode.deadlineExceeded
              : CanonicalErrorCode.commandCancelled,
            timedOut ? "command deadline exceeded" : "command cancelled"
          )
        );
      }
      throw error;
    } finally {
      if (timer !== undefined) {
        clearTimeout(timer);
      }
      options.signal?.removeEventListener("abort", abort);
    }
  }
}

export interface SessionOptions {
  enginePolicy?: EnginePolicy;
  fidelityProfile?: FidelityProfile;
  deadlineMs?: number;
  signal?: AbortSignal;
}

export class MachinaClient {
  constructor(readonly transport: CommandTransport) {}

  async createSession(options: SessionOptions = {}): Promise<Session> {
    const sessionId = newId("session");
    const payload: SessionCreatePayload = {
      engine_policy: options.enginePolicy ?? EnginePolicy.preferCompatible,
      fidelity_profile: options.fidelityProfile ?? FidelityProfile.agent
    };
    const command: CommandEnvelope = {
      ...baseCommand(sessionId, CommandKind.sessionCreateV1, 1_000, "session.create.v1"),
      payload
    };
    await this.execute(command, {
      timeoutMs: options.deadlineMs,
      signal: options.signal
    });
    return new Session(this, sessionId, options);
  }

  async execute(command: CommandEnvelope, options: ExecuteOptions = {}): Promise<CommandOutcome> {
    const outcome = await this.transport.execute(command, options);
    if (outcome.error) {
      throw new MachinaError(outcome.error);
    }
    return outcome;
  }
}

export class Session {
  private closed = false;
  private readonly closeIdempotencyKey = newId("close");
  private closePromise?: Promise<CommandOutcome>;

  constructor(
    private readonly client: MachinaClient,
    readonly id: string,
    private readonly defaults: SessionOptions
  ) {}

  page(pageId = "page-1"): Page {
    return new Page(this.client, this.id, pageId, this.defaults);
  }

  async navigate(url: string, waitUntil: WaitUntil = WaitUntil.load): Promise<CommandOutcome> {
    const payload: NavigationGotoPayload = { url, wait_until: waitUntil };
    return this.client.execute(
      {
        ...baseCommand(this.id, CommandKind.navigationGotoV1, this.defaults.deadlineMs ?? 30_000, "navigation.goto.v1"),
        payload
      },
      executeOptions(this.defaults)
    );
  }

  async close(reason = "client_close"): Promise<CommandOutcome | undefined> {
    if (this.closed) {
      return undefined;
    }
    if (this.closePromise) {
      return this.closePromise;
    }
    const payload: SessionClosePayload = { reason };
    this.closePromise = this.client.execute(
      {
        ...baseCommand(
          this.id,
          CommandKind.sessionCloseV1,
          this.defaults.deadlineMs ?? 30_000,
          "session.close.v1",
          this.closeIdempotencyKey
        ),
        payload
      },
      executeOptions(this.defaults)
    );
    try {
      const outcome = await this.closePromise;
      this.closed = true;
      return outcome;
    } catch (error: unknown) {
      this.closePromise = undefined;
      throw error;
    }
  }

  async *events(options: SubscribeOptions = {}): AsyncIterable<SessionEvent> {
    let after = options.afterSequence ?? 0;
    let attempts = 0;
    const maxAttempts = options.maxReconnectAttempts ?? 3;
    const delayMs = options.reconnectDelayMs ?? 50;
    while (!options.signal?.aborted) {
      try {
        for await (const event of this.client.transport.subscribe(this.id, after, options.signal)) {
          if (event.sequence <= after) {
            continue;
          }
          after = event.sequence;
          attempts = 0;
          yield event;
        }
        if (options.signal?.aborted) {
          return;
        }
        if (attempts >= maxAttempts) {
          throw new Error("event stream ended before cancellation");
        }
        attempts += 1;
        await delay(delayMs * attempts, options.signal);
      } catch (error: unknown) {
        if (
          options.signal?.aborted ||
          (error instanceof MachinaError && !error.canonical.retryable) ||
          attempts >= maxAttempts
        ) {
          throw error;
        }
        attempts += 1;
        await delay(delayMs * attempts, options.signal);
      }
    }
  }
}

export class Page {
  constructor(
    private readonly client: MachinaClient,
    private readonly sessionId: string,
    readonly id: string,
    private readonly defaults: SessionOptions
  ) {}

  async extract(query: string): Promise<CommandOutcome> {
    const payload: SemanticQueryPayload = { query };
    return this.client.execute(
      {
        ...baseCommand(this.sessionId, CommandKind.domSemanticQueryV1, this.defaults.deadlineMs ?? 30_000, "dom.semantic_query.v1"),
        page_id: this.id,
        payload
      },
      executeOptions(this.defaults)
    );
  }

  async click(selector: string): Promise<CommandOutcome> {
    const payload: ClickPayload = { selector };
    return this.client.execute(
      {
        ...baseCommand(this.sessionId, CommandKind.interactionClickV1, this.defaults.deadlineMs ?? 30_000, "interaction.click.v1"),
        page_id: this.id,
        payload
      },
      executeOptions(this.defaults)
    );
  }
}

function baseCommand<K extends CommandKindType>(
  sessionId: string,
  kind: K,
  deadlineMs: number,
  capability: string,
  idempotencyKey = newId("idempotency")
): CommandPrefix<K> {
  return {
    command_id: newId("command"),
    session_id: sessionId,
    kind,
    idempotency_key: idempotencyKey,
    deadline_ms: deadlineMs,
    required_capabilities: [capability],
    metadata: {
      correlation_id: newId("correlation"),
      client: "@machina/sdk-typescript"
    }
  };
}

type CommandPrefix<K extends CommandKindType> = {
  command_id: string;
  session_id: string;
  kind: K;
  context_id?: string;
  page_id?: string;
  idempotency_key?: string;
  deadline_ms: number;
  required_capabilities: string[];
  metadata: {
    correlation_id: string;
    causation_id?: string;
    client: string;
  };
};

function executeOptions(options: SessionOptions): ExecuteOptions {
  return {
    timeoutMs: options.deadlineMs,
    signal: options.signal
  };
}

function newId(prefix: string): string {
  if (typeof globalThis.crypto?.randomUUID === "function") {
    return `${prefix}-${globalThis.crypto.randomUUID()}`;
  }
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function recordField(value: unknown, field: string): unknown {
  if (!isRecord(value)) {
    throw new Error(`response is not an object; expected ${field}`);
  }
  return value[field];
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text();
  if (text.trim().length === 0) {
    return {};
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error("response body is not valid JSON");
  }
}

function parseCanonicalError(value: unknown): CanonicalError {
  if (!isRecord(value) || typeof value.code !== "string" || !isCanonicalErrorCode(value.code)) {
    throw new Error("response did not contain a canonical error");
  }
  const details = value.details;
  if (!isRecord(details)) {
    throw new Error("response field details is invalid");
  }
  return {
    code: value.code,
    category: stringField(value, "category"),
    message: stringField(value, "message"),
    retryable: booleanField(value, "retryable"),
    retry_after_ms: numberOptionalField(value, "retry_after_ms"),
    engine: engineOptionalField(value, "engine"),
    capability: stringOptionalField(value, "capability"),
    command_id: stringField(value, "command_id"),
    correlation_id: stringField(value, "correlation_id"),
    details,
    cause_code:
      typeof value.cause_code === "string" && isCanonicalErrorCode(value.cause_code)
        ? value.cause_code
        : undefined,
    documentation_ref: stringField(value, "documentation_ref")
  };
}

function parseOutcome(value: unknown): CommandOutcome {
  if (!isRecord(value) || typeof value.command_id !== "string") {
    throw new Error("response did not contain a command outcome");
  }
  const execution = value.execution;
  if (!isRecord(execution)) {
    throw new Error("command outcome is missing execution metadata");
  }
  const status = value.status;
  if (typeof status !== "string" || !isOutcomeStatus(status)) {
    throw new Error("command outcome status is invalid");
  }
  return {
    command_id: value.command_id,
    attempt: numberField(value, "attempt"),
    status,
    result: stringOptionalField(value, "result"),
    error: value.error === undefined ? undefined : parseCanonicalError(value.error),
    execution: parseExecution(execution),
    trace_ref: stringOptionalField(value, "trace_ref")
  };
}

function parseExecution(value: Record<string, unknown>): EngineExecution {
  const engine = value.engine;
  if (typeof engine !== "string" || (engine !== EngineKind.native && engine !== EngineKind.chromium)) {
    throw new Error("command execution engine is invalid");
  }
  return {
    requested_engine_policy: stringField(value, "requested_engine_policy"),
    engine,
    engine_version: stringField(value, "engine_version"),
    capability_snapshot: stringField(value, "capability_snapshot"),
    fallback_used: booleanField(value, "fallback_used"),
    fallback_reason: stringOptionalField(value, "fallback_reason"),
    migration_id: stringOptionalField(value, "migration_id")
  };
}

function parseEvent(value: unknown): SessionEvent {
  if (!isRecord(value) || typeof value.sequence !== "number" || typeof value.event_type !== "string") {
    throw new Error("event stream item is invalid");
  }
  return {
    event_id: stringOptionalField(value, "event_id"),
    sequence: value.sequence,
    event_type: value.event_type,
    session_id: stringOptionalField(value, "session_id"),
    payload: stringField(value, "payload"),
    correlation_id: stringOptionalField(value, "correlation_id"),
    timestamp: stringOptionalField(value, "timestamp")
  };
}

async function* parseEventStream(
  body: ReadableStream<Uint8Array>
): AsyncIterable<SessionEvent> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    while (true) {
      const chunk = await reader.read();
      buffer += decoder.decode(chunk.value ?? new Uint8Array(), { stream: !chunk.done });
      const frames = buffer.split(/\r?\n\r?\n/);
      buffer = frames.pop() ?? "";
      for (const frame of frames) {
        const data = frame
          .split(/\r?\n/)
          .filter((line) => line.startsWith("data:"))
          .map((line) => line.slice(5).trim())
          .join("\n");
        if (data.length > 0) {
          const parsed: unknown = JSON.parse(data);
          yield parseEvent(parsed);
        }
      }
      if (chunk.done) {
        return;
      }
    }
  } finally {
    await reader.cancel();
    reader.releaseLock();
  }
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

function syntheticError(code: CanonicalErrorCode, message: string): CanonicalError {
  return {
    code,
    category: "sdk",
    message,
    retryable: code === CanonicalErrorCode.deadlineExceeded,
    command_id: "",
    correlation_id: "",
    details: {},
    documentation_ref: `errors/${code}`
  };
}

function isCanonicalErrorCode(value: string): value is CanonicalErrorCode {
  return Object.values(CanonicalErrorCode).some((candidate) => candidate === value);
}

function isOutcomeStatus(value: string): value is OutcomeStatus {
  return Object.values(OutcomeStatus).some((candidate) => candidate === value);
}

function stringField(value: Record<string, unknown>, field: string): string {
  if (typeof value[field] !== "string") {
    throw new Error(`response field ${field} is invalid`);
  }
  return value[field];
}

function stringOptionalField(value: Record<string, unknown>, field: string): string | undefined {
  return value[field] === undefined ? undefined : stringField(value, field);
}

function numberField(value: Record<string, unknown>, field: string): number {
  if (typeof value[field] !== "number") {
    throw new Error(`response field ${field} is invalid`);
  }
  return value[field];
}

function numberOptionalField(value: Record<string, unknown>, field: string): number | undefined {
  return value[field] === undefined ? undefined : numberField(value, field);
}

function booleanField(value: Record<string, unknown>, field: string): boolean {
  if (typeof value[field] !== "boolean") {
    throw new Error(`response field ${field} is invalid`);
  }
  return value[field];
}

function engineOptionalField(
  value: Record<string, unknown>,
  field: string
): EngineKindType | undefined {
  const engine = value[field];
  if (engine === undefined) {
    return undefined;
  }
  if (engine !== EngineKind.native && engine !== EngineKind.chromium) {
    throw new Error(`response field ${field} is invalid`);
  }
  return engine;
}

function delay(milliseconds: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(resolve, milliseconds);
    signal?.addEventListener(
      "abort",
      () => {
        clearTimeout(timer);
        reject(new MachinaError(syntheticError(CanonicalErrorCode.commandCancelled, "command cancelled")));
      },
      { once: true }
    );
  });
}
