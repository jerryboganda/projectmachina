"""Transport-neutral async client over the canonical command model."""

from __future__ import annotations

import asyncio
import json
import uuid
from dataclasses import dataclass
from enum import Enum
from typing import Any, AsyncIterator, Mapping, Protocol
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen


class CanonicalErrorCode(str, Enum):
    INVALID_ARGUMENT = "INVALID_ARGUMENT"
    UNAUTHENTICATED = "UNAUTHENTICATED"
    PERMISSION_DENIED = "PERMISSION_DENIED"
    POLICY_DENIED = "POLICY_DENIED"
    QUOTA_EXCEEDED = "QUOTA_EXCEEDED"
    RATE_LIMITED = "RATE_LIMITED"
    SESSION_NOT_READY = "SESSION_NOT_READY"
    SESSION_CLOSED = "SESSION_CLOSED"
    SESSION_EXPIRED = "SESSION_EXPIRED"
    CAPACITY_UNAVAILABLE = "CAPACITY_UNAVAILABLE"
    WORKER_LOST = "WORKER_LOST"
    COMMAND_CANCELLED = "COMMAND_CANCELLED"
    DEADLINE_EXCEEDED = "DEADLINE_EXCEEDED"
    UNSUPPORTED_CAPABILITY = "UNSUPPORTED_CAPABILITY"
    CAPABILITY_DISABLED = "CAPABILITY_DISABLED"
    RENDERER_REQUIRED = "RENDERER_REQUIRED"
    FALLBACK_PROHIBITED = "FALLBACK_PROHIBITED"
    MIGRATION_FAILED = "MIGRATION_FAILED"
    STATE_TRANSFER_PARTIAL = "STATE_TRANSFER_PARTIAL"
    INVALID_URL = "INVALID_URL"
    NETWORK_POLICY_BLOCKED = "NETWORK_POLICY_BLOCKED"
    NAVIGATION_FAILED = "NAVIGATION_FAILED"
    SELECTOR_INVALID = "SELECTOR_INVALID"
    ELEMENT_NOT_FOUND = "ELEMENT_NOT_FOUND"
    ELEMENT_AMBIGUOUS = "ELEMENT_AMBIGUOUS"
    ELEMENT_NOT_INTERACTABLE = "ELEMENT_NOT_INTERACTABLE"
    ACTION_POSTCONDITION_FAILED = "ACTION_POSTCONDITION_FAILED"
    WORKFLOW_INVALID = "WORKFLOW_INVALID"
    APPROVAL_REQUIRED = "APPROVAL_REQUIRED"
    SECRET_UNAVAILABLE = "SECRET_UNAVAILABLE"


class EngineKind(str, Enum):
    CHROMIUM = "chromium"
    NATIVE = "native"


class OutcomeStatus(str, Enum):
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    CANCELLED = "cancelled"
    DEADLINE_EXCEEDED = "deadline_exceeded"


@dataclass(frozen=True)
class CanonicalError:
    code: CanonicalErrorCode
    category: str
    message: str
    retryable: bool
    retry_after_ms: int | None
    engine: EngineKind | None
    capability: str | None
    command_id: str
    correlation_id: str
    details: Mapping[str, Any]
    cause_code: CanonicalErrorCode | None
    documentation_ref: str

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "CanonicalError":
        try:
            code = CanonicalErrorCode(value["code"])
            return cls(
                code=code,
                category=_required_string(value, "category"),
                message=_required_string(value, "message"),
                retryable=_required_bool(value, "retryable"),
                retry_after_ms=_optional_int(value, "retry_after_ms"),
                engine=_optional_enum(value, "engine", EngineKind),
                capability=_optional_string(value, "capability"),
                command_id=_required_string(value, "command_id"),
                correlation_id=_required_string(value, "correlation_id"),
                details=_required_mapping(value, "details"),
                cause_code=_optional_enum(value, "cause_code", CanonicalErrorCode),
                documentation_ref=_required_string(value, "documentation_ref"),
            )
        except (KeyError, TypeError, ValueError) as error:
            raise ValueError("response did not contain a canonical error") from error


class MachinaError(Exception):
    """A canonical server or transport failure."""

    def __init__(self, error: CanonicalError, status: int | None = None) -> None:
        super().__init__(error.message)
        self.error = error
        self.code = error.code
        self.status = status


@dataclass(frozen=True)
class EngineExecution:
    requested_engine_policy: str
    engine: EngineKind
    engine_version: str
    capability_snapshot: str
    fallback_used: bool
    fallback_reason: str | None
    migration_id: str | None

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "EngineExecution":
        try:
            return cls(
                requested_engine_policy=_required_string(value, "requested_engine_policy"),
                engine=EngineKind(value["engine"]),
                engine_version=_required_string(value, "engine_version"),
                capability_snapshot=_required_string(value, "capability_snapshot"),
                fallback_used=_required_bool(value, "fallback_used"),
                fallback_reason=_optional_string(value, "fallback_reason"),
                migration_id=_optional_string(value, "migration_id"),
            )
        except (KeyError, TypeError, ValueError) as error:
            raise ValueError("response did not contain engine execution metadata") from error


@dataclass(frozen=True)
class SessionEvent:
    sequence: int
    event_type: str
    payload: str
    event_id: str | None = None
    session_id: str | None = None
    correlation_id: str | None = None
    timestamp: str | None = None

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "SessionEvent":
        sequence = value.get("sequence")
        if not isinstance(sequence, int) or sequence <= 0:
            raise ValueError("event sequence must be positive")
        return cls(
            sequence=sequence,
            event_type=_required_string(value, "event_type"),
            payload=_required_string(value, "payload"),
            event_id=_optional_string(value, "event_id"),
            session_id=_optional_string(value, "session_id"),
            correlation_id=_optional_string(value, "correlation_id"),
            timestamp=_optional_string(value, "timestamp"),
        )


@dataclass(frozen=True)
class CommandOutcome:
    command_id: str
    attempt: int
    status: OutcomeStatus
    result: str | None
    error: CanonicalError | None
    trace_ref: str | None
    execution: EngineExecution
    raw: Mapping[str, Any]

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "CommandOutcome":
        try:
            return cls(
                command_id=_required_string(value, "command_id"),
                attempt=_required_int(value, "attempt"),
                status=OutcomeStatus(value["status"]),
                result=_optional_string(value, "result"),
                error=(
                    CanonicalError.from_mapping(_required_mapping(value, "error"))
                    if value.get("error") is not None
                    else None
                ),
                trace_ref=_optional_string(value, "trace_ref"),
                execution=EngineExecution.from_mapping(
                    _required_mapping(value, "execution")
                ),
                raw=value,
            )
        except (KeyError, TypeError, ValueError) as error:
            raise ValueError("response did not contain a valid command outcome") from error


class AsyncTransport(Protocol):
    async def execute(
        self,
        command: Mapping[str, Any],
        *,
        timeout: float | None = None,
        cancel_event: asyncio.Event | None = None,
    ) -> CommandOutcome:
        ...

    def subscribe(
        self,
        session_id: str,
        after_sequence: int,
        *,
        cancel_event: asyncio.Event | None = None,
    ) -> AsyncIterator[SessionEvent]:
        ...


class HttpTransport:
    """Minimal stdlib HTTP/SSE transport for clean environments."""

    def __init__(
        self,
        base_url: str,
        headers: Mapping[str, str] | None = None,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._headers = dict(headers or {})

    async def execute(
        self,
        command: Mapping[str, Any],
        *,
        timeout: float | None = None,
        cancel_event: asyncio.Event | None = None,
    ) -> CommandOutcome:
        request = Request(
            f"{self._base_url}/v1/commands",
            data=json.dumps(command).encode("utf-8"),
            headers={
                **self._headers,
                "content-type": "application/json",
                "accept": "application/json",
            },
            method="POST",
        )
        operation = asyncio.create_task(
            asyncio.to_thread(_read_http_json, request, timeout)
        )
        try:
            body, status = await _await_operation(operation, timeout, cancel_event)
        except HTTPError as error:
            body = await asyncio.to_thread(_read_error_json, error)
            raise MachinaError(
                CanonicalError.from_mapping(_required_mapping(body, "error")),
                error.code,
            ) from error
        except URLError as error:
            raise RuntimeError("Project Machina transport request failed") from error
        if status < 200 or status >= 300:
            raise MachinaError(CanonicalError.from_mapping(_required_mapping(body, "error")), status)
        return CommandOutcome.from_mapping(body)

    async def _subscribe(
        self,
        session_id: str,
        after_sequence: int,
        cancel_event: asyncio.Event | None,
    ) -> AsyncIterator[SessionEvent]:
        request = Request(
            f"{self._base_url}/v1/sessions/{quote(session_id, safe='')}/events?after_sequence={after_sequence}",
            headers={**self._headers, "accept": "text/event-stream"},
            method="GET",
        )
        try:
            response = await asyncio.to_thread(urlopen, request, timeout=30.0)
        except HTTPError as error:
            body = await asyncio.to_thread(_read_error_json, error)
            raise MachinaError(
                CanonicalError.from_mapping(_required_mapping(body, "error")),
                error.code,
            ) from error
        try:
            while True:
                if cancel_event is not None and cancel_event.is_set():
                    raise MachinaError(_synthetic_error(CanonicalErrorCode.COMMAND_CANCELLED, "command cancelled"))
                line = await _readline_or_cancel(response, cancel_event)
                if not line:
                    return
                if not line.startswith(b"data:"):
                    continue
                data = line[5:].strip()
                if not data:
                    continue
                yield SessionEvent.from_mapping(json.loads(data))
        finally:
            response.close()

    def subscribe(
        self,
        session_id: str,
        after_sequence: int,
        *,
        cancel_event: asyncio.Event | None = None,
    ) -> AsyncIterator[SessionEvent]:
        return self._subscribe(session_id, after_sequence, cancel_event)


class MachinaClient:
    def __init__(self, transport: AsyncTransport) -> None:
        self.transport = transport

    async def create_session(
        self,
        *,
        engine_policy: str = "prefer-compatible",
        fidelity_profile: str = "agent",
        timeout: float | None = 30.0,
        cancel_event: asyncio.Event | None = None,
    ) -> "Session":
        session_id = _new_id("session")
        command = _command(
            session_id,
            "session.create.v1",
            "session.create.v1",
            {
                "engine_policy": engine_policy,
                "fidelity_profile": fidelity_profile,
            },
            1_000,
        )
        await self.execute(command, timeout=timeout, cancel_event=cancel_event)
        return Session(self, session_id, timeout, cancel_event)

    async def execute(
        self,
        command: Mapping[str, Any],
        *,
        timeout: float | None = None,
        cancel_event: asyncio.Event | None = None,
    ) -> CommandOutcome:
        outcome = await self.transport.execute(
            command,
            timeout=timeout,
            cancel_event=cancel_event,
        )
        if outcome.error is not None:
            raise MachinaError(outcome.error)
        return outcome


class Session:
    def __init__(
        self,
        client: MachinaClient,
        session_id: str,
        timeout: float | None,
        cancel_event: asyncio.Event | None,
    ) -> None:
        self._client = client
        self.id = session_id
        self._timeout = timeout
        self._cancel_event = cancel_event
        self._closed = False
        self._close_idempotency_key = _new_id("close")
        self._close_task: asyncio.Task[CommandOutcome] | None = None

    def page(self, page_id: str = "page-1") -> "Page":
        return Page(self._client, self.id, page_id, self._timeout, self._cancel_event)

    async def navigate(self, url: str, *, wait_until: str = "load") -> CommandOutcome:
        return await self._client.execute(
            _command(
                self.id,
                "navigation.goto.v1",
                "navigation.goto.v1",
                {"url": url, "wait_until": wait_until},
                30_000,
            ),
            timeout=self._timeout,
            cancel_event=self._cancel_event,
        )

    async def close(self, reason: str = "client_close") -> CommandOutcome | None:
        if self._closed:
            return None
        if self._close_task is None:
            self._close_task = asyncio.create_task(
                self._client.execute(
                    _command(
                        self.id,
                        "session.close.v1",
                        "session.close.v1",
                        {"reason": reason},
                        30_000,
                        idempotency_key=self._close_idempotency_key,
                    ),
                    timeout=self._timeout,
                    cancel_event=self._cancel_event,
                )
            )
        try:
            outcome = await asyncio.shield(self._close_task)
            self._closed = True
            return outcome
        except asyncio.CancelledError:
            if self._close_task.cancelled():
                self._close_task = None
            raise
        except BaseException:
            self._close_task = None
            raise

    async def events(
        self,
        *,
        after_sequence: int = 0,
        max_reconnect_attempts: int = 3,
        reconnect_delay: float = 0.05,
        cancel_event: asyncio.Event | None = None,
    ) -> AsyncIterator[SessionEvent]:
        after = after_sequence
        attempts = 0
        while cancel_event is None or not cancel_event.is_set():
            try:
                async for event in self._client.transport.subscribe(
                    self.id,
                    after,
                    cancel_event=cancel_event,
                ):
                    if event.sequence <= after:
                        continue
                    after = event.sequence
                    attempts = 0
                    yield event
                if cancel_event is not None and cancel_event.is_set():
                    return
                if attempts >= max_reconnect_attempts:
                    raise RuntimeError("event stream ended before cancellation")
                attempts += 1
                await asyncio.sleep(reconnect_delay * attempts)
            except MachinaError as error:
                if not error.error.retryable:
                    raise
                if cancel_event is not None and cancel_event.is_set():
                    raise
                if attempts >= max_reconnect_attempts:
                    raise
                attempts += 1
                await asyncio.sleep(reconnect_delay * attempts)
            except (OSError, TimeoutError):
                if cancel_event is not None and cancel_event.is_set():
                    raise
                if attempts >= max_reconnect_attempts:
                    raise
                attempts += 1
                await asyncio.sleep(reconnect_delay * attempts)


class Page:
    def __init__(
        self,
        client: MachinaClient,
        session_id: str,
        page_id: str,
        timeout: float | None,
        cancel_event: asyncio.Event | None,
    ) -> None:
        self._client = client
        self._session_id = session_id
        self.id = page_id
        self._timeout = timeout
        self._cancel_event = cancel_event

    async def extract(self, query: str) -> CommandOutcome:
        return await self._client.execute(
            _command(
                self._session_id,
                "dom.semantic_query.v1",
                "dom.semantic_query.v1",
                {"query": query},
                30_000,
                page_id=self.id,
            ),
            timeout=self._timeout,
            cancel_event=self._cancel_event,
        )

    async def click(self, selector: str) -> CommandOutcome:
        return await self._client.execute(
            _command(
                self._session_id,
                "interaction.click.v1",
                "interaction.click.v1",
                {"selector": selector},
                30_000,
                page_id=self.id,
            ),
            timeout=self._timeout,
            cancel_event=self._cancel_event,
        )


async def _await_operation(
    operation: asyncio.Task[tuple[Mapping[str, Any], int]],
    timeout: float | None,
    cancel_event: asyncio.Event | None,
) -> tuple[Mapping[str, Any], int]:
    if cancel_event is None:
        try:
            return await asyncio.wait_for(operation, timeout=timeout)
        except asyncio.TimeoutError as error:
            operation.cancel()
            raise MachinaError(
                _synthetic_error(
                    CanonicalErrorCode.DEADLINE_EXCEEDED,
                    "command deadline exceeded",
                )
            ) from error
    cancellation = asyncio.create_task(cancel_event.wait())
    try:
        done, _ = await asyncio.wait(
            {operation, cancellation},
            timeout=timeout,
            return_when=asyncio.FIRST_COMPLETED,
        )
        if cancellation in done and cancellation.result():
            operation.cancel()
            raise MachinaError(_synthetic_error(CanonicalErrorCode.COMMAND_CANCELLED, "command cancelled"))
        if operation not in done:
            operation.cancel()
            raise MachinaError(_synthetic_error(CanonicalErrorCode.DEADLINE_EXCEEDED, "command deadline exceeded"))
        return operation.result()
    finally:
        cancellation.cancel()


async def _readline_or_cancel(
    response: Any,
    cancel_event: asyncio.Event | None,
) -> bytes:
    read_task = asyncio.create_task(asyncio.to_thread(response.readline))
    if cancel_event is None:
        return await read_task
    cancellation = asyncio.create_task(cancel_event.wait())
    try:
        done, _ = await asyncio.wait(
            {read_task, cancellation},
            return_when=asyncio.FIRST_COMPLETED,
        )
        if cancellation in done and cancellation.result():
            response.close()
            read_task.cancel()
            raise MachinaError(
                _synthetic_error(
                    CanonicalErrorCode.COMMAND_CANCELLED,
                    "command cancelled",
                )
            )
        return read_task.result()
    finally:
        cancellation.cancel()


def _read_http_json(
    request: Request,
    timeout: float | None,
) -> tuple[Mapping[str, Any], int]:
    with urlopen(request, timeout=timeout if timeout is not None else 30.0) as response:
        body = json.loads(response.read().decode("utf-8"))
        if not isinstance(body, Mapping):
            raise ValueError("response is not an object")
        return body, response.status


def _read_error_json(error: HTTPError) -> Mapping[str, Any]:
    body = json.loads(error.read().decode("utf-8"))
    if not isinstance(body, Mapping):
        raise ValueError("error response is not an object")
    return body


def _command(
    session_id: str,
    kind: str,
    capability: str,
    payload: Mapping[str, Any],
    deadline_ms: int,
    *,
    page_id: str | None = None,
    idempotency_key: str | None = None,
) -> dict[str, Any]:
    command: dict[str, Any] = {
        "command_id": _new_id("command"),
        "session_id": session_id,
        "kind": kind,
        "idempotency_key": idempotency_key or _new_id("idempotency"),
        "deadline_ms": deadline_ms,
        "required_capabilities": [capability],
        "metadata": {
            "correlation_id": _new_id("correlation"),
            "client": "project-machina-sdk-python",
        },
        "payload": dict(payload),
    }
    if page_id is not None:
        command["page_id"] = page_id
    return command


def _new_id(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4()}"


def _synthetic_error(code: CanonicalErrorCode, message: str) -> CanonicalError:
    return CanonicalError(
        code=code,
        category="sdk",
        message=message,
        retryable=code is CanonicalErrorCode.DEADLINE_EXCEEDED,
        retry_after_ms=None,
        engine=None,
        capability=None,
        command_id="",
        correlation_id="",
        details={},
        cause_code=None,
        documentation_ref=f"errors/{code.value}",
    )


def _required_string(value: Mapping[str, Any], field: str) -> str:
    result = value.get(field)
    if not isinstance(result, str):
        raise ValueError(f"response field {field} is invalid")
    return result


def _optional_string(value: Mapping[str, Any], field: str) -> str | None:
    result = value.get(field)
    if result is None:
        return None
    if not isinstance(result, str):
        raise ValueError(f"response field {field} is invalid")
    return result


def _optional_int(value: Mapping[str, Any], field: str) -> int | None:
    result = value.get(field)
    if result is None:
        return None
    if not isinstance(result, int):
        raise ValueError(f"response field {field} is invalid")
    return result


def _optional_enum(
    value: Mapping[str, Any],
    field: str,
    enum_type: type[Enum],
) -> Any:
    result = value.get(field)
    if result is None:
        return None
    try:
        return enum_type(result)
    except (TypeError, ValueError) as error:
        raise ValueError(f"response field {field} is invalid") from error


def _required_bool(value: Mapping[str, Any], field: str) -> bool:
    result = value.get(field)
    if not isinstance(result, bool):
        raise ValueError(f"response field {field} is invalid")
    return result


def _required_int(value: Mapping[str, Any], field: str) -> int:
    result = value.get(field)
    if not isinstance(result, int):
        raise ValueError(f"response field {field} is invalid")
    return result


def _required_mapping(value: Mapping[str, Any], field: str) -> Mapping[str, Any]:
    result = value.get(field)
    if not isinstance(result, Mapping):
        raise ValueError(f"response field {field} is invalid")
    return result
