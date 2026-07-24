# GitHub Component

This crate builds the Wasm component for Firna's repo-owned GitHub App. Depend
on it only through the app package runtime ABI; installation verification,
ephemeral token minting, and workspace ownership remain platform duties.

## Responsibilities

- Decode and validate calls for the five GitHub read tools.
- Construct known `api.github.com` REST paths and bounded query parameters.
- Request host-mediated bearer injection without receiving the raw token.
- Convert typed GitHub responses into bounded, stable Firna projections.

## What This Crate Does

The component exports `call-tool` and imports only `host-http-request`. It
allows no arbitrary URL input, emits only `GET` requests, rejects host-truncated
responses before parsing, and maps provider failures to stable app errors. A
stable host `credential_not_found` response becomes `auth_required` for
`github_installation`; provider, vault, network, malformed, and unknown host
failures remain redacted provider-unavailable errors.

Exact file reads accept at most 16 path segments, resolve the requested ref to a
commit, and walk non-recursive Git trees one path segment at a time. Only
regular and executable blob modes are accepted; directories, symlinks, and
submodules are rejected before the Contents API can dereference them. The final
Contents request is pinned to the resolved commit and must return the verified
blob SHA.

## Quick Start

```bash
cargo test --manifest-path apps/github/component/Cargo.toml --locked
cargo build --manifest-path apps/github/component/Cargo.toml \
  --target wasm32-unknown-unknown --locked
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

## Development

Component unit tests mock the `GitHubProvider` and `Clock` traits with
`unimock`. Keep provider fixtures in tests, keep production DTOs fully typed,
and preserve the 768 KiB projection budget beneath the manifest's 1 MiB host
limit.

### Key Code

- `src/lib.rs` defines the WIT imports and `call-tool` export.
- `src/github/input.rs` and `input_validation.rs` own input contracts.
- `src/github/models/` owns provider DTOs.
- `src/github/tools/` owns request construction and projections, including the
  commit-pinned file tree walk in `repository_file.rs`.
- `src/github/provider_response.rs` owns provider status and rate-limit mapping.

### Related Docs

- [`../README.md`](../README.md)
- [GitHub App protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/github-app.md)
- [GitHub tool protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/github-app-tools.md)
