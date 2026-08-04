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
- Verify priced-result decoding, exact bounded usage, and typed uncharged
  failures.

## What This Crate Does

Read tests cover bounded Post lookup and one-page recent search. Write tests
prove that one component invocation sends at most one create request and never
forwards Firna's operation id. The OAuth lifecycle test uses the credential-
scoped host to replace a rejected bearer token and retry the exact request once.
Platform-owned refresh-token rotation and durable operation-ledger behavior are
covered at the pinned platform boundary. The platform store proves that a
completed execution is replayed as the same single result and is no longer
pending; reclaimed agent turns consume that stored tool history; and an
ambiguous pending installed-app execution fails closed without reconstructing
or invoking the app tool. Together with this crate's one-request X write test,
those regressions prove that recovery cannot send a second create-Post request.
Runtime assertions also prove that usage is separated from the agent-visible
output, empty search results report zero units, and text and link creation
report their exact manifest-capped costs.

## Quick Start

Once the reviewed platform refresh change is merged and this crate's manifest
is pinned to that revision, run:

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
crate runs `x_component_never_retries_an_ambiguous_write`; platform revision
`90d4c13f` runs
`tool_execution_transitions_from_pending_to_completed`,
`reclaimed_event_replays_normalized_tool_history_without_provider_context`, and
`pending_tool_executions_fail_closed_without_reexecution`. Keeping the durable
ledger in the platform avoids a second X-specific journal with conflicting
recovery semantics.

### Key Code

- `src/lib.rs` builds the package and component used by every test.
- `x_read_smoke_tests.rs` and `x_write_smoke_tests.rs` cover all three tools.
- `x_error_tests.rs` covers provider failures, truncation, and redaction.
- `x_oauth_lifecycle_tests.rs` covers host-owned bearer refresh and retry.

### Related Docs

- [X package](../../README.md)
- [X component](../../component/README.md)
- [X app protocol](../../../../docs/protocol/x-app.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
