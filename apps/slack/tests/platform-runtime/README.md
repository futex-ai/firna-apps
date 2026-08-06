# Slack Platform Runtime Tests

This crate verifies the Slack component against the pinned Firna Wasm runtime.
Depend on it only as a standalone integration-test package.

## Responsibilities

- Build and wrap the Slack WebAssembly component reproducibly.
- Validate OAuth, tool, webhook, and event manifest contracts.
- Exercise Slack requests and signature handling through a fake host.

## What This Crate Does

The tests run the real component through `fna-apps-wasm` without live Slack
credentials or network calls. They cover tool calls, webhook normalization,
message events, error behavior, and package documentation.

## Quick Start

```sh
cargo test --manifest-path apps/slack/tests/platform-runtime/Cargo.toml --locked
```

## Development

Install `wasm32-unknown-unknown` and the `wasm-tools` version from the root
`platform.toml` before running the suite.

### Key Code

- `src/lib.rs` builds and wraps the component.
- `slack_manifest_tests.rs` validates package metadata and event declarations.
- `slack_package_tests.rs` covers host HTTP and webhook runtime behavior.
- `slack_url_verification_tests.rs` covers Slack verification responses.

### Related Docs

- [Slack package](../../README.md)
- [Repository development](../../../../README.md#development)
