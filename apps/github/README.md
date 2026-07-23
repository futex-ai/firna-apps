# GitHub App

`apps/github` is Firna's first-party GitHub App package. A workspace admin
installs it for a GitHub account, chooses all or selected repositories in
GitHub, and exposes a bounded, read-only repository tool set to permitted
workspace users.

## Responsibilities

- Declare the built-in, workspace-owned GitHub App installation flow and its
  exact read-only repository permissions.
- Restrict component HTTP access to `GET https://api.github.com/...` with an
  host-minted installation credential bound to `github_installation`.
- Build five typed tools for repository listing, code search, exact file reads,
  pull requests, and issues.
- Keep provider responses, credentials, and projections within manifest and
  component limits.

## What This App Does

The workspace owns one verified GitHub App installation. Trusted server code
uses a short-lived user token only to prove that the approving admin can access
the selected installation, then discards it. The host signs bounded app JWTs
and mints one-hour installation tokens for metadata, contents, issues, and pull
requests, all read-only. Tokens never reach the component or persistent
storage. Provider requests use only `GET` and never follow redirects.

The tool interface is:

- `github_list_repositories`: list repositories selected for the installation.
- `github_search_code`: search visible code with bounded literal qualifiers.
- `github_read_file`: return one exact UTF-8 file up to 256 KiB.
- `github_read_pr`: read PR detail and one bounded changed-files page.
- `github_read_issue`: read issue detail and one bounded comments page.

## Quick Start

```bash
firna apps validate apps/github
firna apps package apps/github
cargo build --manifest-path apps/github/component/Cargo.toml \
  --target wasm32-unknown-unknown --locked
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

Local provider testing requires a separately registered GitHub App and an
uncommitted manifest override. Never commit its client secret or RSA private
key, or put either value below `apps/github`. Deployment refuses this package
while a public registration sentinel remains or either required Secret Manager
value cannot be loaded.

The production GitHub App uses slug `firna`, setup URL
`https://firna.ai/apps/github/install/callback`, and authorization callback URL
`https://firna.ai/apps/github/authorize/callback`. Its public client ID belongs
in `manifest.yaml`; `client_secret` and `private_key` remain deployment secrets.
The workspace installation credential is identified by
`github_installation`.

## Development

Run component unit tests before the platform-runtime suite:

```bash
cargo test --manifest-path apps/github/component/Cargo.toml --locked
```

The component tests use provider fixtures only. Platform-runtime tests compile
the component and exercise its real Wasm ABI with a mocked trusted host; they
do not call GitHub.

### Key Code

- `manifest.yaml` owns GitHub App registration metadata, permissions, HTTP
  policy, and tool schemas.
- Its string schemas expose compatible owner, repository, language, path, and
  ref constraints; component validation retains stricter UTF-8 byte, Unicode
  control-character, trimming, and path-segment checks.
- `component/src/github/provider.rs` defines the mocked provider boundary.
- `component/src/github/tools/` validates calls and builds typed projections.
- `component/src/github/host.rs` emits only known GitHub REST requests with the
  virtual installation credential reference.
- `tests/platform-runtime/` verifies package metadata and Wasm host behavior.

### Related Docs

- [GitHub App protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/github-app.md)
- [GitHub tool protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/github-app-tools.md)
- [App platform protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [App deployment](https://github.com/futex-ai/firna/blob/main/docs/deployment/apps.md)
