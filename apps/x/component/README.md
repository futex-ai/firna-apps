# X Component

This crate builds the Wasm component for the repo-owned X app. Depend on it
only as the package component; installation, OAuth exchange, credential
storage, refresh, and durable tool recovery belong to the Firna platform.

## Responsibilities

- Decode and validate the three declared X tool calls.
- Build bounded lookup, recent-search, and create-Post requests.
- Request opaque host bearer-token injection for the workspace installation.
- Return compact typed results, bounded usage reports, and stable redacted
  failures.

## What This Crate Does

The component exports `call-tool` and imports only `host-http-request`. Its HTTP
boundary is a trait, so unit tests use `unimock` without network or credential
access. Reads issue one provider request for one bounded page. Create issues at
most one request and maps an ambiguous transport result to
`write_outcome_unknown`. Successful calls use the platform's priced envelope:
reads report returned Post and User counts, while creation reports the
manifest-capped text or link rate. Typed failures remain outside that envelope
and therefore cannot be charged.

## Quick Start

```bash
cargo fmt --manifest-path apps/x/component/Cargo.toml -- --check
cargo clippy --manifest-path apps/x/component/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path apps/x/component/Cargo.toml --locked
cargo build --manifest-path apps/x/component/Cargo.toml --target wasm32-unknown-unknown --locked
```

## Development

Production and test source files must remain below 300 lines. Provider JSON is
converted into typed DTOs at the HTTP boundary, and provider bodies or tokens
must never appear in returned errors.

### Key Code

- `src/lib.rs` defines the WIT import and `call-tool` export.
- `src/x/service.rs` owns validation and request construction.
- `src/x/host.rs` owns the opaque host HTTP trait and request DTOs.
- `src/x/response.rs` owns provider status and response mapping.
- `src/x/_tests_/service` covers reads, writes, and error behavior.

### Related Docs

- [X package](../README.md)
- [X app protocol](../../../docs/protocol/x-app.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
