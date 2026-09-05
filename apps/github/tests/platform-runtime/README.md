# GitHub Platform Runtime Tests

This crate verifies the GitHub package against the Firna app manifest and Wasm
runtime pinned by this repository. Depend on it only as a standalone
integration-test package.

## Responsibilities

- Build and wrap the GitHub WebAssembly component reproducibly.
- Validate installation, permission, tool, webhook, event, secret, and limit
  manifest contracts.
- Exercise GitHub requests and signed webhooks through a fake trusted host.

## What This Crate Does

The tests run the real component through `fna-apps-wasm` without live GitHub
credentials or network calls. They cover all five tools, credential references,
request construction, provider failures, bounded file traversal, HMAC host
calls, duplicate headers, ping, lifecycle classification, and all six event
projection families. It also verifies all 16 published definitions, all 28
acknowledge-and-drop definitions, their exact permissions and effect
declarations, and the package-to-platform reconciliation contract.

## Quick Start

```sh
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

## Development

Install `wasm32-unknown-unknown` and the `wasm-tools` version from the root
`platform.toml` before running the suite.

### Key Code

- `src/lib.rs` registers the standalone integration-test modules.
- `github_runtime_support.rs` builds and wraps the component.
- `github_package_tests.rs` validates package metadata and schemas.
- `github_tool_smoke_tests.rs` exercises the tool host boundary.
- `github_file_smoke_tests.rs` verifies commit-pinned tree checks.
- `github_webhook_smoke_tests.rs` exercises signed event behavior.
- `github_acknowledgement_conformance_tests.rs` proves every dormant event and
  its retries bypass normalization and durable acceptance.
- `github_webhook_manifest_tests.rs` checks the complete event and permission
  catalog.
- `github_effect_conformance_tests.rs` exercises signed Wasm normalization,
  atomic platform acceptance, effect dispatch, a provider status re-read,
  snapshot replacement, and post-commit update publication.

### Related Docs

- [GitHub package](../../README.md)
- [Repository development](../../../../README.md#development)
