# X App

`apps/x` is the repo-owned X integration package. It gives an explicitly
authorized Firna workspace bounded Post lookup, recent search, and one-Post
publishing without exposing OAuth tokens to the component or agents.

## Responsibilities

- Declare workspace-owned OAuth 2.0 with PKCE, offline access, and host-owned
  refresh-token rotation.
- Expose only the three reviewed V1 tools and keep every billed request bounded.
- Build a Wasm component that validates inputs and returns compact typed data.
- Keep client credentials and workspace tokens behind opaque host credentials.
- Report bounded usage so successful calls settle against the workspace wallet.

## What This App Does

| Tool | Behavior |
| --- | --- |
| `x_get_posts` | Reads 1-10 Post ids; author expansion is opt-in. |
| `x_search_recent_posts` | Reads one explicit 10-25-result recent-search page. |
| `x_create_post` | Publishes one text Post or reply with no component retry. |

The app requests `tweet.read`, `tweet.write`, `users.read`, and
`offline.access`. It does not expose media, threads, deletion, editing,
streams, automatic pagination, or background polling.

## Quick Start

```bash
firna apps validate apps/x
firna apps package apps/x
cargo build --manifest-path apps/x/component/Cargo.toml --target wasm32-unknown-unknown --locked
cargo test --manifest-path apps/x/component/Cargo.toml --locked
cargo test --manifest-path apps/x/tests/platform-runtime/Cargo.toml --locked
```

All commands use the canonical platform revision recorded in
[`platform.toml`](../../platform.toml).

These commands validate locally and do not deploy. This repository's deploy
workflow releases to production and the stable `br-main` preview after a
reviewed change merges to `main` (`apps/x/deploy.toml` targets the
`production` and `preview` classes); labelled PR previews exclude X because
arbitrary per-PR callbacks are not registered.

Use separate confidential Web Apps in the X Developer Console. Register
`https://firna.ai/oauth/x/callback` for production and
`https://br-main.preview.firna.ai/oauth/x/callback` for stable preview. Supply
both required manifest values through Google Secret Manager:

- production: `prod-app-x-client-id` and `prod-app-x-client-secret`;
- stable preview: `preview-app-x-client-id` and
  `preview-app-x-client-secret`.

Both live in the dedicated app-secrets Google Cloud project defined by
[`docs/protocol/app-deployment.md`](../../docs/protocol/app-deployment.md).

Never add either credential value to this package.

## Cost and Write Safety

Firna prepays X and charges the authorizing workspace only after a successful
tool call. Post reads cost $0.005 per returned Post, expanded User reads cost
$0.010 per returned author, text-only creation costs $0.015, and link-bearing
creation costs $0.200. Failed calls are uncharged. Prices are fixed for app
version `1.0.5`; changing them requires a new version and explicit workspace
consent.

Production requires both platform billing and app charging to be enabled. If
either is disabled, or the wallet cannot cover the declared worst-case hold,
the call fails before X receives a request.

X may deduplicate repeated resource reads within a UTC day, but that upstream
decision is not visible per response, so Firna charges each returned resource.
Author expansion is off by default, link-bearing text requires
`allow_link: true`, and each component invocation sends at most one provider
request. Firna's durable operation ledger owns completed-result replay and
fail-closed recovery for ambiguous pending writes. Current provider prices and
the developer-account spending limit must still be checked before credits are
purchased.

## Development

### Key Code

- `manifest.yaml` owns OAuth, tool schemas, host permissions, and runtime limits.
- `component/src/x/service.rs` validates calls and builds bounded requests.
- `component/src/x/response.rs` maps typed provider responses to stable results.
- `tests/platform-runtime` exercises the built component through the pinned host.

### Related Docs

- [X app protocol](../../docs/protocol/x-app.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [X API pricing](https://docs.x.com/x-api/getting-started/pricing)
- [X OAuth user access](https://docs.x.com/fundamentals/authentication/oauth-2-0/user-access-token)
