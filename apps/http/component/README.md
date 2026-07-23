# HTTP Component

The HTTP component implements the `firna-app-component-v1` `call-tool` export
for `http_request`. It performs request-shape validation and delegates all
network access to the host `host-http-request` import.

## Responsibilities

- Validate model-supplied HTTP methods, URLs, headers, bodies, and timeouts.
- Delegate outbound requests to the capability-enforcing Firna host.
- Normalize bounded provider responses without exposing credential headers.

## What This Crate Does

The component exports `call-tool` for the first-party HTTP app. It has no
ambient network or secret access and can reach model-selected hosts only
through the host import authorized by the built-in manifest.

## Quick Start

```bash
cargo test --manifest-path apps/http/component/Cargo.toml --locked
cargo build --manifest-path apps/http/component/Cargo.toml --target wasm32-unknown-unknown --release --locked
```

## Development

Create a component wrapper for local inspection after the release build:

```bash
http_component_target="${CARGO_TARGET_DIR:-apps/http/component/target}"
wasm-tools component new \
  "$http_component_target/wasm32-unknown-unknown/release/fna_app_http_component.wasm" \
  -o /tmp/fna_app_http_component.wasm
```

### Key Code

- `src/http/tools.rs` validates and renders requests.
- `src/http/host.rs` owns the typed host HTTP boundary.
- `src/http/types.rs` owns request and response DTOs.

### Related Docs

- [Package README](../README.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
