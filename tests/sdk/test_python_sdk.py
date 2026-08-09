from __future__ import annotations

import asyncio
import unittest
from collections.abc import AsyncIterator, Mapping

from machina_sdk import (
    AsyncTransport,
    CanonicalError,
    CanonicalErrorCode,
    EngineExecution,
    EngineKind,
    MachinaClient,
    MachinaError,
    OutcomeStatus,
    SessionEvent,
)
from machina_sdk.client import CommandOutcome


class FakeTransport(AsyncTransport):
    def __init__(self) -> None:
        self.commands: list[Mapping[str, object]] = []
        self.subscribe_attempts = 0

    async def execute(
        self,
        command: Mapping[str, object],
        *,
        timeout: float | None = None,
        cancel_event: asyncio.Event | None = None,
    ) -> CommandOutcome:
        self.commands.append(command)
        return CommandOutcome(
            command_id=str(command["command_id"]),
            attempt=1,
            status=OutcomeStatus.SUCCEEDED,
            result="ok",
            error=None,
            trace_ref="trace-1",
            execution=EngineExecution(
                requested_engine_policy="prefer-compatible",
                engine=EngineKind.CHROMIUM,
                engine_version="fixture",
                capability_snapshot="capability.v0",
                fallback_used=False,
                fallback_reason=None,
                migration_id=None,
            ),
            raw={
                "status": "succeeded",
                "execution": {
                    "requested_engine_policy": "prefer-compatible",
                    "engine": "chromium",
                    "engine_version": "fixture",
                    "capability_snapshot": "capability.v0",
                    "fallback_used": False,
                },
            },
        )

    async def _subscribe(
        self,
        session_id: str,
        after_sequence: int,
        cancel_event: asyncio.Event | None,
    ) -> AsyncIterator[SessionEvent]:
        self.subscribe_attempts += 1
        if self.subscribe_attempts == 1:
            raise ConnectionError("fixture disconnect")
        yield SessionEvent(
            sequence=after_sequence + 1,
            event_type="session.lifecycle.v1",
            payload="{}",
            session_id=session_id,
        )

    def subscribe(
        self,
        session_id: str,
        after_sequence: int,
        *,
        cancel_event: asyncio.Event | None = None,
    ) -> AsyncIterator[SessionEvent]:
        return self._subscribe(session_id, after_sequence, cancel_event)


class SdkTests(unittest.IsolatedAsyncioTestCase):
    async def test_session_page_close_and_reconnecting_events(self) -> None:
        transport = FakeTransport()
        session = await MachinaClient(transport).create_session()
        await session.navigate("https://fixture.local/")
        await session.page().extract("main")
        await session.close()
        self.assertEqual(
            [command["kind"] for command in transport.commands],
            [
                "session.create.v1",
                "navigation.goto.v1",
                "dom.semantic_query.v1",
                "session.close.v1",
            ],
        )
        events = []
        cancel_event = asyncio.Event()
        async for event in session.events(reconnect_delay=0, cancel_event=cancel_event):
            events.append(event)
            cancel_event.set()
        self.assertEqual(events[0].sequence, 1)
        self.assertEqual(transport.subscribe_attempts, 2)

    async def test_canonical_errors_are_typed(self) -> None:
        error = CanonicalError(
            code=CanonicalErrorCode.RENDERER_REQUIRED,
            category="protocol",
            message="renderer unavailable",
            retryable=True,
            retry_after_ms=None,
            engine=None,
            capability=None,
            command_id="command-1",
            correlation_id="correlation-1",
            details={},
            cause_code=None,
            documentation_ref="errors/RENDERER_REQUIRED",
        )
        self.assertEqual(MachinaError(error).code, CanonicalErrorCode.RENDERER_REQUIRED)
        with self.assertRaises(ValueError):
            CommandOutcome.from_mapping(
                {
                    "command_id": "command-1",
                    "attempt": 1,
                    "status": "succeeded",
                }
            )


if __name__ == "__main__":
    unittest.main()
