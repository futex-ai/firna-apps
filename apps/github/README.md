# GitHub

`apps/github` is Firna's built-in GitHub App package. It declares the
workspace-owned installation flow used to mint short-lived, repository-scoped
credentials for connected external repositories.

## Responsibilities

- Declare the GitHub App registration identity and exact installation-token
  permissions.
- Restrict provider HTTP access to bounded `GET` and `POST` requests to
  `api.github.com`.
- Keep the App private key, client secret, JWTs, and minted installation tokens
  inside the trusted host credential path.
- Ship no agent tools or webhook ingress in the baseline credential package.

## What This App Does

A workspace administrator installs the Firna GitHub App on a GitHub account
and selects repositories there. A workspace can connect additional accounts as
separate installations. Firna verifies each installation and can mint a
one-hour token for one selected repository when an environment needs Git
access. The baseline package exposes no agent tools and leaves webhooks
disabled; those capabilities can be added in later package versions without
changing the repository credential boundary.

The registration uses these public routes:

- Setup URL: `https://firna.ai/apps/github/install/setup`
- Callback URL: `https://firna.ai/apps/github/install/callback`

The production registration is owned by the `Firna-AI` GitHub organization:

- App ID: `4504159`
- Client ID: `Iv23lidBdZ0I2rgwjhXB`
- Slug: `firna-ai`
- Public page: `https://github.com/apps/firna-ai`

## Quick Start

```sh
firna apps validate apps/github
firna apps package apps/github
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

## Development

Run the repository verifier after any manifest, component, or registration
metadata change:

```sh
cargo xtask check
```

`client_secret` and `private_key` are required deployment secrets. Never put
their values in this repository, a command argument, or a developer-built Wasm
artifact.

### Key Code

- `manifest.yaml` owns installation identity, permissions, HTTP limits, and
  callback URLs.
- `component/src/lib.rs` provides the required component ABI while the package
  intentionally declares no tools.
- `tests/platform-runtime/` validates the manifest and component against the
  platform revision pinned by the repository.

### Related Docs

- [App package conventions](../README.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [External repository protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/external-repos.md)
