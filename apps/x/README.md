# X App

`apps/x` is the repo-owned X integration package. It gives an explicitly
authorized Firna workspace bounded access to X Posts, accounts, timelines,
engagement, relationships, Lists, Spaces, Communities, trends, media, and
Direct Messages without exposing OAuth tokens to the component or agents.

Package `2.0.2` opts into the platform's generic multi-connection OAuth
contract, so one workspace can authorize several independently managed X
accounts without giving the component or agent access to their tokens.

## Responsibilities

- Declare workspace-owned OAuth 2.0 with PKCE, offline access, and host-owned
  per-connection refresh-token rotation.
- Identify each authorization with one trusted, bounded `GET /2/users/me`
  before tokens are published, mapping X user id to immutable connection
  identity and username to public display label.
- Opt into the platform's 25-account workspace limit while keeping add,
  reconnect, disable, agent access, refresh, and disconnect installation-scoped.
- Expose 23 reviewed domain tools and keep every billed request bounded.
- Use an app-owned bearer secret only for X endpoints that reject user tokens.
- Build a Wasm component that validates inputs and returns compact typed data.
- Keep client credentials and workspace tokens behind opaque host credentials.
- Report bounded usage so successful calls settle against the workspace wallet.

## What This App Does

| Area | Tools and behavior |
| --- | --- |
| Posts | Lookup and metrics, recent/full search, counts, engagement, feeds, expanded create/edit, and Post actions. |
| Accounts | Self/id/username lookup, profile search, affiliates, relationships, and follow/mute/DM-block actions. |
| Lists | List lookup and collections plus create, update, delete, member, follow, and pin actions. |
| Discovery | Space lookup/search/Posts/buyers, Community lookup/search, and personalized or location trends. |
| Messaging | Bounded DM event reads, sends, group creation, deletion, and bookmark-folder creation. |
| Media | Metadata lookup plus alt-text and subtitle management for existing media ids. |

The manifest requests only the OAuth scopes consumed by these modes; the exact
list is in the [X app protocol](../../docs/protocol/x-app.md). Full-archive
search, Post counts, and location trends use a deployment-owned app bearer.
Profile clicks are clicks from one Post to its author's profile, not total
profile views. The app does not expose binary media upload, Ads/Enterprise
analytics, streams, automatic pagination, or background polling.

Post search and count queries accept X API v2 engagement operators as well as
the common X web-search aliases: `min_faves:` is translated to `min_likes:`,
and `min_retweets:` to `min_reposts:` before the provider request.

### Multiple Accounts

Workspace owners and admins install the first X account, then use **Connect
another account** to authorize more. Each account is a separate Firna
installation with its own opaque credentials, refresh claim, status, agent
access, billing attribution, and audit identity. Duplicate X user ids are
rejected without rotating the existing account. Reconnect must return the same
X user id as the selected connection, so authorizing a different account cannot
silently replace it. Disconnect removes only that account; the final disconnect
makes the app unavailable until it is installed again.

Every X tool call explicitly selects one agent-visible `connection_id`; the
host shows the corresponding X username, validates the selection, and strips
the reserved field before invoking this component. The component retains its
current input schemas and never receives an account selector or token. It never
chooses a default connection, automatically cross-posts, or fans one call out
across accounts.

For `x_get_user_feed`, omitting `user_id` targets the selected connection. The
component resolves that account with one bounded `/2/users/me` request before
reading the requested feed page. Supplying a user id retains the direct
one-request path.

X does not document an account-forcing parameter for this OAuth 2.0 authorize
endpoint. Before approving another connection, use X's account switcher or
sign out and sign into the intended account in the system browser. If the
current account is returned and Firna reports that it is already connected,
switch accounts in X and start a fresh authorization. Do not add OAuth 1.0a's
`force_login` or `screen_name` parameters to this OAuth 2.0 manifest.

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
- app-only API access: `prod-app-x-bearer-token` and
  `preview-app-x-bearer-token`, respectively.

Both live in the dedicated app-secrets Google Cloud project defined by
[`docs/protocol/app-deployment.md`](../../docs/protocol/app-deployment.md).

Never add either credential value to this package.

## Cost and Write Safety

Firna prepays X and charges the authorizing workspace only after a successful
tool call. Reads report X's documented resource category, and actions report
their exact successful request cost; the immutable per-unit schedule and every
worst-case hold are defined in the manifest and
[protocol](../../docs/protocol/x-app.md). Text-only creation costs $0.015 and
link-bearing creation costs $0.200. Failed calls are uncharged. Changing any
price requires a new version and explicit workspace consent.

The one `/2/users/me` identity request made during install, add, reconnect, or
upgrade is control-plane work, not a billed tool call. Firna absorbs any X cost
for it and includes that bounded overhead in the provider spending limit. A
feed call that omits `user_id` makes a separate data-plane identity lookup and
reports it as one User read.

Production requires both platform billing and app charging to be enabled. If
either is disabled, or the wallet cannot cover the declared worst-case hold,
the call fails before X receives a request.

X may deduplicate repeated resource reads within a UTC day, but that upstream
decision is not visible per response, so Firna charges each returned resource.
Author expansion is off by default and link-bearing text requires
`allow_link: true`. Calls normally send one provider request; a feed without a
user id sends one identity lookup followed by one bounded page request. Firna's
durable operation ledger owns completed-result replay and fail-closed recovery
for ambiguous pending writes. Current provider prices and the developer-account
spending limit must still be checked before credits are purchased.

## Development

### Key Code

- `manifest.yaml` owns OAuth, tool schemas, host permissions, and runtime limits.
- [`assets/README.md`](assets/README.md) documents the icon source and how to
  regenerate the PNG, its base64 sidecar, and the embedded manifest value.
- `component/src/x/service` validates calls and builds bounded requests.
- `component/src/x/metrics_types.rs` owns the typed provider and metric output
  boundary.
- `component/src/x/response.rs` maps typed provider responses to stable results.
- `tests/platform-runtime` exercises the built component through the pinned host.

### Related Docs

- [X app protocol](../../docs/protocol/x-app.md)
- [X Posts and feeds](../../docs/protocol/x-app-posts.md)
- [X accounts and relationships](../../docs/protocol/x-app-accounts.md)
- [X Lists and discovery](../../docs/protocol/x-app-lists-discovery.md)
- [X Direct Messages](../../docs/protocol/x-app-messaging.md)
- [X Post metrics](../../docs/protocol/x-app-metrics.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [X API pricing](https://docs.x.com/x-api/getting-started/pricing)
- [X OAuth user access](https://docs.x.com/fundamentals/authentication/oauth-2-0/user-access-token)
- [X authenticated user](https://docs.x.com/x-api/users/get-my-user)
- [X OAuth API reference](https://docs.x.com/fundamentals/authentication/api-reference)
