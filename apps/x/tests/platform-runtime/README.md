# X Platform Runtime Tests

This crate verifies the repo-owned X package against the Firna Wasm runtime.
Depend on it only as a standalone conformance suite; provider logic belongs to
the component, while credential refresh and durable recovery belong to the
platform.

## Responsibilities

- Build and convert the real X component into a Wasm component.
- Validate the package manifest, icon, OAuth lifecycle, schemas, and limits.
- Exercise every X tool through the real Wasm runtime and mocked host imports.
- Verify exact provider requests, stable redacted errors, and OAuth host retry.
- Prove two installation ids inject distinct bearer tokens and refreshing one
  connection does not rotate or retry the other.
- Verify priced-result decoding, exact bounded usage, and typed uncharged
  failures.

## What This Crate Does

Read tests cover all declared domains, app-only credential routing, bounded
Post lookup, public and opt-in private Post metrics, and one-page search. They
distinguish missing private fields from real zeroes and verify exact metered
usage. Write tests smoke every added action through the real Wasm boundary and
prove that one invocation never forwards Firna's operation id or retries an
ambiguous request. The OAuth lifecycle test uses the credential-
scoped host to replace a rejected bearer token and retry the exact request once.
Platform-owned refresh-token rotation and durable operation-ledger behavior are
covered at the pinned platform boundary. The platform store proves that a
completed execution is replayed as the same single result and is no longer
pending; reclaimed agent turns consume that stored tool history; and an
ambiguous pending installed-app execution fails closed without reconstructing
or invoking the app tool. Together with this crate's one-request X write test,
those regressions prove that recovery cannot send a second create-Post request.
The pinned platform OAuth suite also proves that duplicate identities are
rejected before credential publication and that a targeted reconnect cannot
replace one connection with another provider account. Its settings suite keeps
the remaining live connections available after one connection is disconnected.
Runtime assertions also prove that usage is separated from the agent-visible
output, empty search results report zero units, and text and link creation
report their exact manifest-capped costs.

## Quick Start

Run against the canonical platform revision pinned by this repository:

```bash
cargo fmt --manifest-path apps/x/tests/platform-runtime/Cargo.toml -- --check
cargo clippy --manifest-path apps/x/tests/platform-runtime/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path apps/x/tests/platform-runtime/Cargo.toml --locked
```

The suite needs `wasm32-unknown-unknown` and `wasm-tools` because it builds and
converts the real component before runtime assertions execute.

## Development

No test contacts X, reads a real credential, or incurs provider charges. Test
responses are fixtures at the host boundary, and every output assertion rejects
provider-only fields or secret-bearing error details.

The durable write proof is intentionally split at the ownership boundary. This
crate runs `x_component_never_retries_an_ambiguous_write`; the platform
revision pinned in the repository's `platform.toml` runs
`tool_execution_transitions_from_pending_to_completed`,
`reclaimed_event_replays_normalized_tool_history_without_provider_context`, and
`pending_tool_executions_fail_closed_without_reexecution`. Keeping the durable
ledger in the platform avoids a second X-specific journal with conflicting
recovery semantics.

### Key Code

- `src/lib.rs` builds the package and component used by every test.
- `x_coverage_smoke_tests.rs` invokes every expanded tool through real Wasm.
- `x_read_smoke_tests.rs`, `x_metrics_smoke_tests.rs`, and
  `x_write_smoke_tests.rs` retain the original-tool regressions.
- `x_package/` verifies the 23-tool schema, OAuth scope, and pricing catalog.
- `x_error_tests.rs` covers provider failures, truncation, and redaction.
- `x_metrics_error_tests.rs` applies those failure guarantees to metrics reads.
- `x_oauth_lifecycle_tests.rs` covers host-owned bearer refresh and retry.
- `x_connection_routing_tests.rs` covers installation-scoped bearer isolation.

### Related Docs

- [X package](../../README.md)
- [X component](../../component/README.md)
- [X app protocol](../../../../docs/protocol/x-app.md)
- [X Post metrics protocol](../../../../docs/protocol/x-app-metrics.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
