# Session control service foundation

M1-T01 owns the durable control-plane schema and repository contract. Workers
must not write these tables directly; they use scoped service interfaces and
transactional outbox events. Every query carries organization/project scope,
and every aggregate transition uses an optimistic version.

The SQL migrations are transport-neutral and can be applied by the eventual
session-control service. The Rust `machina-control-plane` crate provides the
dependency-free contract and fast transaction/idempotency tests.
