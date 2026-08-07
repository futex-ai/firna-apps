# GitHub

`apps/github` is Firna's built-in GitHub App package. It supplies
repository-scoped installation credentials, bounded read tools, and signed
repository events to workspaces that explicitly install it.

## Responsibilities

- Preserve the GitHub App installation-token boundary used by external
  repository environments.
- Expose five read-only agent tools without exposing installation tokens to the
  component or model.
- Authenticate GitHub webhooks before routing, lifecycle work, normalization,
  persistence, or subscriber delivery.
- Keep private keys, client secrets, webhook secrets, JWTs, and minted tokens
  inside trusted host credential paths.

## What This App Does

A workspace administrator installs the Firna GitHub App on a GitHub account and
selects repositories in GitHub. Each connected GitHub-side installation is a
separate Firna app installation. The platform mints short-lived,
repository-scoped tokens for external repository work and installation-scoped
tokens for these read tools. Every tool requires the `github_installation`
authorization declared by the package.

| Tool | Inputs | Result |
| --- | --- | --- |
| `github_list_repositories` | `page?`, `per_page?` (maximum 50) | One bounded page of repositories selected for the installation. |
| `github_search_code` | `query`, paired `owner?` and `repository?`, `language?`, `path?`, `page?`, `per_page?` (maximum 20) | Literal code matches within GitHub's first 1,000 search results. |
| `github_read_file` | `owner`, `repository`, `path`, `ref?` | One commit-pinned regular UTF-8 file of at most 256 KiB and 16 path segments. |
| `github_read_pr` | `owner`, `repository`, `number`, `include_files?`, `files_page?`, `files_per_page?` (maximum 10) | Pull request details and an optional bounded changed-file page. |
| `github_read_issue` | `owner`, `repository`, `number`, `include_comments?`, `comments_page?`, `comments_per_page?` (maximum 10) | Issue details and an optional bounded comment page. |

All five tools are declared `external_read`. The existing Contents and Pull
requests write permissions remain available only to the platform's external
repository workflow; the component emits only `GET` requests.

The `github_events` ingress publishes six native events for explicit agent
subscription:

- `push`
- `pull_request`
- `pull_request_review`
- `pull_request_review_comment`
- `issues`
- `issue_comment`

Installing GitHub does not subscribe or wake an agent. GitHub's implicit
`ping`, `installation`, and `installation_repositories` deliveries are
authenticated but are not subscribable content events. Ping receives a minimal
acknowledgement. Installation creation, restoration, permission changes, and
repository-selection changes invalidate cached tokens and reconcile coverage;
suspension and deletion revoke provider access.

The GitHub App registrations are owned by the `Firna-AI` organization.
Production uses:

- App ID: `4504159`
- Client ID: `Iv23lidBdZ0I2rgwjhXB`
- Slug: `firna-ai`
- Public page: <https://github.com/apps/firna-ai>
- Setup URL: <https://firna.ai/apps/github/install/setup>
- Callback URL: <https://firna.ai/apps/github/install/callback>
- Webhook URL: <https://api.firna.ai/apps/github/webhooks/github_events>

The stable `br-main` preview uses its own registration:

- App ID: `4515873`
- Client ID: `Iv23liSZsLmwSZrxxpzm`
- Slug: `firna-ai-preview`
- Public page: <https://github.com/apps/firna-ai-preview>
- Setup URL: <https://br-main.preview.firna.ai/apps/github/install/setup>
- Callback URL: <https://br-main.preview.firna.ai/apps/github/install/callback>
- Webhook URL: <https://br-main.api.preview.firna.ai/apps/github/webhooks/github_events>

Both registrations must match the manifest: Contents write, Issues read,
Metadata read, and Pull requests write. Select only Push, Pull request, Pull
request review, Pull request review comment, Issues, and Issue comment as
configurable webhook events; GitHub sends installation lifecycle events
implicitly. The package targets production and the stable preview, but excludes
ephemeral `pr-N` previews because their callback and webhook URLs are not
registered.

The manifest declares seven deployment-owned values:

- `app_slug`
- `callback_url`
- `client_id`
- `client_secret`
- `private_key`
- `setup_url`
- `webhook_secret`

The deployment supplies `app_slug`, `callback_url`, `client_id`, and
`setup_url` for the target registration. They are public registration values,
but use the app-owned environment boundary so the same package can run in both
environments. The remaining three values are sensitive and must stay in Secret
Manager.

The webhook secret must be a high-entropy value shared only with the GitHub App
registration. Firna accepts exactly one `x-hub-signature-256`,
`x-github-delivery`, and `x-github-event` value, verifies HMAC-SHA256 over
the unchanged UTF-8 body with a constant-time comparison, and rejects malformed,
duplicate, unsigned, oversized, or event-disagreeing input.

## Quick Start

```sh
firna apps validate apps/github
firna apps package apps/github
cargo test --manifest-path apps/github/component/Cargo.toml --locked
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

## Development

Build the same Wasm target used by the package builder, then run the repository
verifier:

```sh
cargo build --manifest-path apps/github/component/Cargo.toml \
  --target wasm32-unknown-unknown --locked
cargo xtask check
```

Never place secret values in this repository, command arguments, fixtures, or
developer-built Wasm. For a non-production smoke, use a separate GitHub App and
disposable private repository. Verify signed ping, one lifecycle delivery, one
content event, duplicate redelivery, and an altered signature without recording
payload contents or credentials.

Webhook rotation requires updating Firna's `webhook_secret` and the GitHub App
registration in one maintenance window, then redelivering ping and one content
event. Restore both previous values together if verification fails.

### Key Code

- `manifest.yaml` owns registration metadata, permissions, tools, ingress,
  events, secrets, and runtime limits.
- `component/src/github/tools/` owns the five read tools.
- `component/src/github/webhook_validation.rs` owns signed delivery
  verification and lifecycle classification.
- `component/src/github/webhook_projection.rs` owns bounded event
  normalization.
- `tests/fixtures/webhooks/` contains credential-free provider payloads.
- `tests/platform-runtime/` verifies the package through the pinned platform
  Wasm host.

### Related Docs

- [GitHub app protocol](../../docs/protocol/github-app.md)
- [App package conventions](../README.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [External repository protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/external-repos.md)
