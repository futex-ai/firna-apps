# X App Protocol

Status: implemented by X package `2.0.1` on the platform revision pinned in
`platform.toml`.

## Purpose

The first-party `x` app gives explicitly authorized workspace agents bounded
access to X posts, accounts, timelines, engagement, bookmarks, relationships,
Lists, Spaces, Communities, trends, media metadata and management, Direct
Messages, and account actions. Every provider request is initiated by one tool
call or the minimum OAuth lifecycle work needed to keep one connection usable.

The app is explicit-install, priced, and supports 1 through 25 independently
authorized X accounts per workspace. One call always selects exactly one
agent-visible connection. It never fans out across accounts, auto-paginates,
polls, or starts background provider work.

Tool contracts are split by domain:

- [Posts and feeds](x-app-posts.md)
- [Accounts and relationships](x-app-accounts.md)
- [Lists and discovery](x-app-lists-discovery.md)
- [Direct Messages](x-app-messaging.md)
- [Post metrics](x-app-metrics.md)

## Package and Authorization

The package contract is:

- id/name/version: `x`, `X`, `2.0.1`
- source/install/connection: `built_in`, `explicit`, `multiple`
- provider hosts and methods: `api.x.com`; `GET`, `POST`, `PUT`, `DELETE`
- OAuth owner: workspace
- authorization URL: `https://x.com/i/oauth2/authorize`
- token URL and client auth: `https://api.x.com/2/oauth2/token` with
  `client_secret_basic`
- PKCE: required `S256`; refresh window: five minutes
- identity: `GET /2/users/me`, mapping `data.id` and `data.username`
- OAuth credentials: opaque `access_token` and `refresh_token`
- required app secrets: `client_id`, `client_secret`, and `bearer_token`

The OAuth requirement requests exactly these implemented permissions:

`tweet.read`, `tweet.write`, `users.read`, `follows.read`, `follows.write`,
`like.read`, `like.write`, `list.read`, `list.write`, `block.read`, `mute.read`,
`mute.write`, `bookmark.read`, `bookmark.write`, `dm.read`, `dm.write`,
`space.read`, `timeline.read`, `tweet.moderate.write`, `media.write`, and
`offline.access`.

The workspace OAuth access token serves user-context tools. Full-archive Post
search and all-history Post counts require X app-only authentication. The
package also keeps recent counts and location trends in that same public,
app-context boundary. Those modes use the app-owned `bearer_token` and never
fall back to a connected account token. Production reads
`prod-app-x-bearer-token`; stable preview reads
`preview-app-x-bearer-token`. Secret values never enter source, component
memory, logs, prompts, tool input, or output.

## OAuth and Multiple Accounts

Before publishing new or refreshed tokens, the trusted host calls `/2/users/me`
with the candidate access token. The immutable X user id identifies the
connection and the current username labels it. Duplicate ids are rejected;
reconnect must return the selected connection's stored id. Token refresh,
disable, sharing, and disconnect remain installation-scoped.

Every agent tool receives a required host-owned `connection_id`. The host
validates and removes it before invoking the component, binds the invocation to
that installation, and resolves only credentials allowed by the manifest. The
component receives the opaque installation UUID but never a token. The
app-owned bearer reference deliberately omits installation identity and can
resolve only the manifest-declared app secret.

X documents no OAuth 2.0 account-forcing parameter. Administrators use X's
browser account switcher before authorizing another account. OAuth 1.0a
`force_login` and `screen_name` parameters are not part of this flow.

## Common Data and Pagination

Post, user, List, Space, Community, media, DM, and provider resource ids are
strings, never JSON numbers. Decimal X ids match `^[0-9]{1,19}$`; media keys
match `^[0-9]+_[0-9]+$`. Optional fields absent from X are omitted rather than
invented or emitted as `null`.

New paged tools accept one explicit `max_results` from 10 through 25 and an
optional non-blank `pagination_token` of at most 1,024 bytes. Existing recent
search retains its `next_token` field. A success returns only the current page,
its `result_count`, and a provider token when another page exists. Empty pages
are successful with zero usage unless a lookup contract explicitly requires a
resource and returns `not_found`.

Compact Posts contain `id`, `text`, and optional `author_id` and `created_at`.
Account profiles contain provider-returned identity, bio, verification,
protection, profile URL/image, location, creation time, and public counts.
Domain documents define the remaining compact result types and mode-specific
collections.

## Errors and Write Recovery

Handled failures return `ok: false` with stable codes: `invalid_request`,
`auth_required`, `missing_scope`, `not_found`, `rate_limited`,
`provider_budget_exhausted`, `provider_unavailable`,
`provider_contract_error`, or `write_outcome_unknown`. Safe fields are limited
to a validation reason, auth id, missing scope, or retry delay.

Validation reasons identify the invalid field family or incompatible action
shape; otherwise-unmapped provider 4xx responses use
`provider_rejected_request`. Raw provider bodies, credentials, request
signatures, developer-account ids, and billing identifiers never reach the
agent.

Reads map network loss and 5xx responses to `provider_unavailable`; malformed,
missing, or truncated 2xx bodies map to `provider_contract_error`. Every write
issues at most one provider request. Transport loss, missing status, 5xx,
truncation, or a malformed success after dispatch becomes
`write_outcome_unknown`. Firna's durable operation ledger replays completed
results and fails crash-ambiguous pending writes closed without reinvocation.

## Pricing and Limits

X uses pay-per-use pricing. The manifest declares these public units:

| Unit | Price |
| --- | ---: |
| Post or analytics read | $0.005 per returned resource |
| User, relationship, DM-event, or trend read | $0.010 per resource |
| List, Space, Community, or media read | $0.005 per resource |
| Content create | $0.015, or $0.200 when Post text contains a URL |
| User/DM interaction create | $0.015 per request |
| Interaction delete | $0.010 per request |
| Content/List/Bookmark/Media manage | $0.005 per request |
| List create or privacy update | $0.010 per request |
| Recent/all count request | $0.005/$0.010 per request |

Each tool declares a finite cap from its maximum page size or most expensive
action. The component reports only validated successful resources or the exact
successful action cost. Errors settle at zero. Provider-side daily
deduplication is not observable per response, so Firna charges each returned
resource at the installed version's declared price.

Every tool and provider response is capped at 262,144 bytes and 30 seconds.
The provider spending limit remains the hard account-level control. Prices are
immutable for package `2.0.1` and require explicit update consent.

## Deliberate Exclusions

Synchronous tools exclude streams, webhooks, account-activity subscriptions,
compliance jobs, and background collection. Enterprise analytics, encrypted X
Chat key management, broadcasts, Articles, News, and Community Notes
remain excluded until production access and exact pricing are verified.

X media upload needs multipart file construction. The pinned attachment bridge
streams only an exact raw attachment body and does not expose bytes to the
component, so it cannot safely form X's multipart `media` field. The app can
read media metadata, attach existing media ids to a Post, and use `media.write`
for JSON-only alt text and subtitle management, but it does not pretend to
upload files.

Local tests never contact X or deploy. Production and stable preview release
through their normal `main` workflows after review, with separate OAuth apps,
callbacks, secrets, prepaid credit, and spending limits.
