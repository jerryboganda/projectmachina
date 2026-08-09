# API gateway adapter boundary

M1-T07 exposes the initial transport contract:

- HTTP `POST /v1/commands` maps to `machina-command-bus`.
- gRPC `CommandService.Execute` and `SubscribeEvents` are defined in
  `proto/machina/control/v1/control.proto`.
- Event streams resume after a sequence and apply bounded backpressure through
  `machina-protocol-events`.
- Every adapter must authenticate before resource lookup, preserve the
  canonical correlation/command IDs, and return engine/capability/fallback
  metadata without implementing browser semantics.

The network listener, TLS termination, and production auth middleware are
separate deployment work; this contract layer never bypasses policy.
