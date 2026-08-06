# GitHub Platform Runtime Tests

This crate verifies the GitHub package against the Firna app manifest and Wasm
runtime pinned by this repository.

## Responsibilities

- Parse and validate the credential-only GitHub manifest.
- Assert exact installation permissions, callback URLs, and HTTP bounds.
- Build and compile the real component through the pinned Wasm runtime.

## What This Crate Does

The tests exercise package structure without making live GitHub requests or
reading deployment secrets.

## Quick Start

```sh
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

## Development

Install the Rust target and `wasm-tools` version recorded in the repository's
`platform.toml` before running the suite.

### Key Code

- `src/lib.rs` builds and wraps the component.
- `github_package_tests.rs` asserts the installation-token contract.
- `github_component_tests.rs` compiles the component ABI.

### Related Docs

- [GitHub package](../../README.md)
- [Repository development](../../../../README.md#development)
