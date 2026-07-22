# DataForSEO Platform Runtime Tests

This crate verifies the DataForSEO component against the pinned Firna Wasm
runtime. Depend on it only as a standalone integration-test package.

## Responsibilities

- Build and wrap the DataForSEO WebAssembly component reproducibly.
- Validate its manifest and all model-visible tool schemas.
- Exercise provider request and response behavior through a fake host.

## What This Crate Does

The tests run the real component through `fna-apps-wasm` without live
credentials or network calls. The platform dependencies are pinned to the
repository compatibility revision.

## Quick Start

```sh
cargo test --manifest-path apps/dataforseo/tests/platform-runtime/Cargo.toml --locked
```

## Development

Install `wasm32-unknown-unknown` and the `wasm-tools` version from the root
`platform.toml` before running the suite.

### Key Code

- `src/lib.rs` builds and wraps the component.
- `dataforseo_package_tests.rs` validates package metadata.
- `dataforseo_tool_smoke_tests.rs` exercises the host/runtime boundary.

### Related Docs

- [DataForSEO package](../../README.md)
- [Repository development](../../../../README.md#development)
