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
  persistence, platform effects, or subscriber delivery.
- Publish a bounded 16-event agent catalog and acknowledge 28 additional
  repository-engineering events without retaining their payloads.
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

All five tools are declared `external_read`, emit only `GET`, and declare the
narrow installation-token permission subset they need. Broad registration
grants remain available to platform workflows without being handed to each
read operation.

The `github_events` ingress publishes 16 native events for explicit agent
subscription. Thirteen may also emit a provider-neutral repository-change
effect that causes Firna to re-read current pull-request status; the remaining
three are subscriber-only. Another 28 exact event definitions are authenticated
and acknowledged without normalization, persistence, effects, or delivery.
The complete 16/28 matrix lives in the protocol below.

The subscriber-only definitions are `pull_request_review_comment`, `issues`,
and `issue_comment`. Repository-change producers cover source pushes, pull
request and review state, merge queues, CI/check/status changes, and repository
policy changes. Tag-only pushes remain publishable agent events but do not
trigger platform reconciliation.

Installing GitHub does not subscribe or wake an agent. GitHub's `ping`,
`installation`, `installation_repositories`, `installation_target`, and
mandatory `github_app_authorization` controls are authenticated but never
subscribable. Lifecycle changes invalidate cached tokens and reconcile or
revoke coverage and authorization as defined by the protocol.

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

Both registrations use the exact 16-permission map and 44-event baseline in
the protocol. The original six checkboxes remain the pre-rollout state; add the
other 38 plus `installation_target` only after the compatible Firna platform,
package, and provider grants are active. Leave `meta` and global
`security_advisory` off. The package targets production and stable preview but
excludes ephemeral `pr-N` previews because their URLs are not registered.

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
- `component/src/github/webhook_validation.rs` owns signed common-envelope and
  acknowledge-only classification; the `webhook_*_validation.rs` family
  modules own published and lifecycle shape checks.
- `component/src/github/webhook_projection.rs` routes bounded normalization to
  the content and signal family projections; `webhook_effects.rs` emits the
  provider-neutral repository-change hints.
- `tests/fixtures/webhooks/` contains credential-free provider payloads.
- `tests/platform-runtime/` verifies the package through the pinned platform
  Wasm host.

### Related Docs

- [GitHub app protocol](../../docs/protocol/github-app.md)
- [App package conventions](../README.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [External repository protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/external-repos.md)
