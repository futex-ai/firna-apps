# fna-app-dataforseo-component

This crate is the isolated WebAssembly component for Firna's built-in
DataForSEO app. Depend on it only when building or testing the repo-owned app
package; platform callers execute the built component through `fna-apps-wasm`.

## Responsibilities

- Validate the 16 closed model-visible input schemas.
- Build one bounded DataForSEO Live request per tool call.
- Normalize provider envelopes into compact typed JSON outputs.
- Request host-mediated HTTP Basic auth through installation-scoped opaque
  credential references.

## What This Crate Does

The component dispatches `call-tool`, sends HTTPS requests only to the reviewed
DataForSEO host, checks HTTP plus general and task status codes, and discards
provider request echoes, account data, raw messages, and unknown nested fields.
It lower-cases response header names once at the envelope boundary before
extracting retry and rate-limit metadata, preserving HTTP case-insensitivity
across host implementations.
Keyword normalization admits only the reviewed informational, navigational,
commercial, and transactional intent values in direct and probability output.
It has no filesystem, network, clock, or secret access outside Firna host
imports.

## Quick Start

```sh
cargo build --manifest-path apps/dataforseo/component/Cargo.toml \
  --target wasm32-unknown-unknown --release --locked
cargo test --manifest-path apps/dataforseo/component/Cargo.toml --locked
```

## Development

Run unit tests for validation, request construction, envelope mapping, and
normalization before the package-level platform runtime suite.

### Key Code

- `src/dataforseo/tools/` owns per-product request and output mapping.
- `src/dataforseo/envelope.rs` owns provider status and cost handling.
- `src/dataforseo/host.rs` owns the typed host HTTP boundary.
- `src/dataforseo/input/` owns closed input DTOs and normalization.

### Related Docs

- [Package README](../README.md)
- [DataForSEO tool contract](https://github.com/futex-ai/firna/blob/main/docs/protocol/dataforseo-app-tools.md)
- [App component ABI](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
