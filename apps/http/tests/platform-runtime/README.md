# HTTP Platform Runtime Tests

This crate verifies the HTTP component against the pinned Firna Wasm runtime.
Depend on it only as a standalone integration-test package.

## Responsibilities

- Build and wrap the HTTP WebAssembly component reproducibly.
- Validate its built-in manifest and arbitrary-host capability.
- Exercise request normalization, response bounds, and credential redaction.

## What This Crate Does

The tests run the real component through `fna-apps-wasm` with a fake host; no
live network requests are made.

## Quick Start

```sh
cargo test --manifest-path apps/http/tests/platform-runtime/Cargo.toml --locked
```

## Development

Install `wasm32-unknown-unknown` and the `wasm-tools` version from the root
`platform.toml` before running the suite.

### Key Code

- `src/lib.rs` builds and wraps the component.
- `http_package_tests.rs` validates package metadata.
- `http_tool_smoke_tests.rs` exercises the host/runtime boundary.

### Related Docs

- [HTTP package](../../README.md)
- [Repository development](../../../../README.md#development)
