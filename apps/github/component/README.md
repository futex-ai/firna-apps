# GitHub Component

This crate supplies the Wasm component ABI for Firna's credential-only GitHub
App package. Depend on it only when building or testing that package.

## Responsibilities

- Export the app platform's `call-tool` component function.
- Return a stable invalid-request result if a tool call reaches this package.
- Import no host functions while the baseline package has no tools or ingress.

## What This Crate Does

The package's useful behavior is the trusted-host installation-token flow, not
component code. The small component keeps the package structurally valid and
fails closed if an undeclared tool is invoked.

## Quick Start

```sh
cargo build --manifest-path apps/github/component/Cargo.toml \
  --target wasm32-unknown-unknown --locked
cargo test --manifest-path apps/github/component/Cargo.toml --locked
```

## Development

Keep this component free of provider logic until the manifest declares a
component-owned tool or ingress boundary.

### Key Code

- `src/lib.rs` defines the component world and fail-closed export.
- `src/_tests_/component_tests.rs` verifies the stable response.

### Related Docs

- [GitHub package](../README.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
