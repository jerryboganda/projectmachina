# API gateway adapter boundary

M1-T07 exposes the initial transport contract, and M1-T08 supplies the durable
delivery semantics:

- HTTP `POST /v1/commands` maps to `machina-command-bus`.
- gRPC `CommandService.Execute` and `SubscribeEvents` are defined in
  `proto/machina/control/v1/control.proto`.
- Event streams use a monotonic sequence per session. `EventBroker` keeps a
  bounded replay history and an independently bounded queue for every
  subscriber.
- Acknowledge removes only delivered subscriber entries. A reconnect can resume
  from a sequence while it remains in history; an evicted gap or slow-reader
  overflow returns `ResyncRequired` instead of silently dropping events.
- Consumers use a stable event ID with `IdempotentConsumer` (or the durable
  `event_delivery` projection) so outbox replay cannot repeat a durable effect.
  PostgreSQL consumers call `machina_begin_event_delivery(...)` and apply their
  projection in the same transaction; duplicate calls return `false`, while a
  rollback leaves the delivery eligible for retry.
- Every adapter must authenticate before resource lookup, preserve the
  canonical correlation/command IDs, and return engine/capability/fallback
  metadata without implementing browser semantics.

The network listener, TLS termination, and production auth middleware are
separate deployment work; this contract layer never bypasses policy.
