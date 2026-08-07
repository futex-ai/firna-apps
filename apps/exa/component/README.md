# Exa Component

This crate builds the Wasm component used by the repo-owned Exa app package.
It is intentionally scoped to provider request shaping and host import calls;
platform install, default-app reconciliation, and credential storage live in
Firna app crates.

## Responsibilities

- Decode `exa_web_search` app tool calls.
- Validate search limits before calling the provider.
- Render Exa's camelCase `/search` request body.
- Request host-mediated `x-api-key` injection without knowing whether the host
  selected a workspace-owned key or the app-owned fallback.

## What This Crate Does

The component exports `call-tool` for the app runtime ABI. It imports only
`host-http-request`, so neither API-key source is passed through component
input or output. A rejected workspace key becomes a typed authentication
failure so the user can update or remove it in the Exa app's Settings page.

## Quick Start

```bash
cargo build --manifest-path apps/exa/component/Cargo.toml --target wasm32-unknown-unknown --locked
cargo test --manifest-path apps/exa/tests/platform-runtime/Cargo.toml --locked
```

## Development

### Key Code

- `src/lib.rs` defines the WIT imports and exported `call-tool` function.
- `src/exa/tools.rs` validates inputs and builds provider requests.
- `src/exa/host.rs` sends host-mediated HTTP requests.
- `src/exa/types.rs` contains the app and provider DTOs.

### Related Docs

- [`../README.md`](../README.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
