# GitHub Platform Runtime Tests

This crate verifies the GitHub component against the pinned Firna Wasm
runtime. Depend on it only as a standalone integration-test package.

## Responsibilities

- Build and wrap the GitHub WebAssembly component reproducibly.
- Validate the installation flow, permissions, and tool manifest contracts.
- Exercise GitHub requests and response projections through a fake host.

## What This Crate Does

The tests run the real component through `fna-apps-wasm` without live GitHub
credentials or network calls. They cover package metadata, credential
references, request construction, provider failures, all five tools, and the
runtime error contract. File tests cover the bounded Git tree walk and the
tree-mode guard that prevents symlinks from being read as regular files.

## Quick Start

```sh
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

## Development

Install `wasm32-unknown-unknown` and the `wasm-tools` version from the root
`platform.toml` before running the suite.

### Key Code

- `src/lib.rs` builds and wraps the component.
- `github_package_tests.rs` validates package metadata and runtime behavior.
- `github_file_smoke_tests.rs` verifies commit-pinned Git tree file checks.
- `github_tool_smoke_tests.rs` exercises the host/runtime boundary.

### Related Docs

- [GitHub package](../../README.md)
- [Repository development](../../../../README.md#development)
