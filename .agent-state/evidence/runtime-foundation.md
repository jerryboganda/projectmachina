# Evidence — Runtime foundation continuation

## Implemented

- `machina-capability`: engine-aware capability snapshots with explicit
  eligibility for native, Chromium, hybrid, limited, experimental, disabled,
  and unsupported states.
- `machina-command-bus`: one typed internal dispatch route with fallback policy,
  deadline/cancellation checks, capability selection, explicit fallback
  metadata, and canonical error conversion.
- `machina-session`: session/page lifecycle, cancellation propagation, bounded
  page/request/byte/artifact accounting, and typed limit failures.

## Invariants

- Protocol adapters must call the command bus rather than an engine directly.
- A missing or disabled capability rejects; it never reports success.
- Fallback reports requested policy, actual engine, snapshot, reason, and
  fallback-used state.
- Session resource limits are checked before counters are committed.

## Validation

- `cargo fmt --all -- --check`: not run; terminal execution unavailable.
- `cargo check --workspace`: not run; terminal execution unavailable.
- `cargo test --workspace`: not run; terminal execution unavailable.
