# X App Protocol

## Purpose

The first-party `x` app lets an explicitly authorized Firna workspace read a
bounded set of X posts, search X's recent post index, and publish one text post
or reply. Every provider request is initiated by a tool call or by the minimum
OAuth token lifecycle work needed to keep the connection usable.

V1 is an explicit-install, first-party public-catalog app. It charges the
workspace credit wallet at declared X list rates for successful calls while
Firna's developer account prepays X. Production deployment remains blocked
until that account has prepaid credit and a human-approved billing-cycle
spending limit. One workspace installation owns one shared X authorization;
per-member grants are out of scope.

## Package and Authorization

The package contract is:

- manifest id and name: `x` and `X`
- version: `1.0.2`
- source kind: `built_in`
- install policy: `explicit`
- HTTP allowlist: `api.x.com` only
- OAuth owner: `workspace`
- authorization URL: `https://x.com/i/oauth2/authorize`
- token URL: `https://api.x.com/2/oauth2/token`
- client authentication: `client_secret_basic`
- PKCE: required, `S256`
- scope separator: one ASCII space
- scopes: `tweet.read`, `tweet.write`, `users.read`, `offline.access`
- app-owned secret: `client_secret`, deployed from
  `firna-prod-app-x-client-secret`

The public client id is manifest environment data. The client secret, access
token, and refresh token never enter source, bundles, logs, component input, or
tool output. The host injects only an opaque credential reference into provider
HTTP requests.

The standard OAuth response maps `$.access_token` to `access_token`,
`$.refresh_token` to `refresh_token`, `$.scope` to granted scopes, and
`$.expires_in` to the access-token expiry. The token lifecycle declares the
access-token and refresh-token credential kinds and a five-minute proactive
refresh window. Refresh reuses the reviewed token URL and client-auth method.
It atomically replaces the access token and expiry, replaces the refresh token
when X returns a rotated value, and retains neither superseded value.

Only one refresh may run for an installation at a time. Calls arriving during
refresh wait for that result and then use the current opaque credential. After
an observed authentication rejection, the host may refresh and retry the
provider request once. `invalid_grant`, a missing refresh token, or a terminal
refresh rejection disables usable authorization and produces `auth_required`.
Refresh bodies and credentials are always redacted.

## Common Data Contract

Post ids and reply ids are decimal strings matching `^[0-9]{1,19}$`; they are
never JSON numbers. A compact post has `id`, `text`, and, when returned by X,
`author_id` and `created_at`. A compact author has `id`, `name`, and `username`.
Absent optional fields are omitted rather than emitted as `null`.

Successful read outputs may contain:

- `posts`: ordered compact posts returned by X
- `authors`: compact expanded authors, only when requested
- `missing_ids`: requested ids X did not return, only when non-empty
- `next_token`: an X pagination token, only when another page exists
- `result_count`: the number of posts in this response

Successful creation returns `post` with the provider-confirmed `id` and
`text`. Components do not synthesize provider data or metrics.

Handled failures use the platform app-error contract with `ok: false`, a stable
`error` code, and only the code-specific safe fields. The component reports
invalid requests, missing authorization or scopes, rate limits, provider
budget exhaustion, provider unavailability, invalid provider responses, and
ambiguous writes without raw provider text. The host turns these responses
into typed tool failures before attempting priced-result decoding.

The stable raw component codes and host results are:

| Component code | Host result |
| --- | --- |
| `invalid_request` | typed invalid request with a stable reason code |
| `auth_required` | typed authorization required for `x_workspace` |
| `missing_scope` | typed missing `tweet.read` or `tweet.write` scope |
| `not_found` | stable runtime rejection |
| `rate_limited` | typed rate limit |
| `provider_budget_exhausted` | typed provider budget exhaustion |
| `provider_unavailable` | typed provider unavailability |
| `provider_contract_error` | typed provider contract failure |
| `write_outcome_unknown` | stable fail-closed runtime rejection |

`invalid_request` includes a stable reason: `malformed_tool_call`,
`unknown_tool`, `invalid_post_ids`, `invalid_search_query`,
`invalid_search_page_size`, `invalid_pagination_token`, `invalid_post_text`,
`invalid_reply_target`, `link_acknowledgement_required`, or
`provider_rejected_request`. The last reason represents an otherwise-unmapped
provider 4xx and is non-retryable. `rate_limited` may add
`retry_after_seconds` derived from an X response header.
No handled failure includes a raw provider body, token, request signature,
developer account id, or billing identifier. Unknown 4xx responses become
`invalid_request` with `provider_rejected_request` and no provider text;
timeouts and 5xx responses become `provider_unavailable` for reads. A create
timeout, transport loss, missing HTTP status, 5xx response, or malformed,
missing, or truncated success body after dispatch becomes
`write_outcome_unknown`.

## `x_get_posts`

Input:

- `ids`: required array of 1-10 unique post-id strings
- `include_authors`: optional boolean, default `false`

The component sends one `GET https://api.x.com/2/tweets` request with the ids
joined in request order and `tweet.fields=author_id,created_at,text`. When
`include_authors` is true it also sends `expansions=author_id` and
`user.fields=id,name,username`; otherwise it requests no user expansion.

The output contains `posts`, optional `authors`, optional `missing_ids`, and
`result_count`. A completely missing id set becomes `not_found`; a provider
partial result remains a success with `missing_ids`.

## `x_search_recent_posts`

Input:

- `query`: required non-blank string, at most 512 Unicode scalar values
- `max_results`: required integer from 10 through 25
- `next_token`: optional non-blank X pagination token, at most 1,024 bytes
- `include_authors`: optional boolean, default `false`

The component sends one `GET https://api.x.com/2/tweets/search/recent` request
with the explicit query and page size, optional pagination token, and
`tweet.fields=author_id,created_at,text`. Author expansion uses the same
opt-in parameters as `x_get_posts`.

The output contains `posts`, `result_count`, and optional `authors` and
`next_token`. The component never follows `next_token`; each page requires a
new tool invocation.

## `x_create_post`

Input:

- `text`: required non-blank string, at most 280 Unicode scalar values
- `reply_to_post_id`: optional post-id string
- `allow_link`: optional boolean, default `false`

The component sends one `POST https://api.x.com/2/tweets` with JSON `text` and,
for a reply, `reply.in_reply_to_tweet_id`. It rejects text containing a
case-insensitive `http://` or `https://` sequence unless `allow_link` is true.
This acknowledgement is a cost warning, not a billing oracle; the X console is
authoritative.

The component never automatically retries this endpoint. X does not document
an idempotency key for post creation, so Firna does not forward its durable
`operation_id` as one. The agent runtime durably records the operation before
dispatch, replays an already completed result from its ledger, and fails a
crash-ambiguous pending call closed without invoking the component again. A
caller that receives `write_outcome_unknown` or the runtime's fail-closed
interruption result must inspect X before deliberately issuing a new operation.

## Firna Usage Charges

The manifest prices all three tools and therefore uses `source.kind: built_in`;
V1 platform policy rejects priced community packages. Firna takes a bounded
wallet hold before invoking X, settles only a successful result, and releases
the hold without charging on component, provider, timeout, or malformed-result
failure. If app charging is disabled or the wallet lacks enough spendable
credit, the provider request is not sent.

Read tools report metered units from the validated provider response:

- `post_read`: $0.005 (5,000 micro-USD) for each returned Post
- `user_read`: $0.010 (10,000 micro-USD) for each returned expanded author

`x_get_posts` caps both units at 10 per call. `x_search_recent_posts` caps both
at 25. Missing requested ids and empty search pages add no read units. X's
daily resource deduplication is not observable in an individual response, so
Firna charges each returned resource at the declared app rate even when X may
deduplicate its upstream charge.

`x_create_post` reports its successful call cost with a $0.200 cap. A text-only
Post reports $0.015 (15,000 micro-USD); input containing case-insensitive
`http://` or `https://` reports $0.200 (200,000 micro-USD). The same detector
requires `allow_link: true`, making the price visible in the caller's action.
X remains authoritative if it classifies content differently, and Firna bears
any uncharged provider cost.

Every successful component result uses the priced envelope
`{"output": <tool output>, "usage": <report>}`. The host removes `usage` before
returning `output` to the agent. Prices are immutable for app version `1.0.2`;
any price change requires a new version and explicit workspace update consent.

## Limits and Cost Controls

Each tool declares a 262,144-byte raw response limit and a 30-second component
limit. The component also stops reading a provider response at 262,144 bytes.
Oversized, malformed, or truncated read JSON becomes `provider_contract_error`;
the equivalent create response becomes `write_outcome_unknown` because X may
already have accepted the Post. There is no background polling, streaming,
webhook, automatic pagination, or scheduled read behavior.

Author expansion is opt-in because expanded users are separate billable
resources. Defaults omit engagement metrics and user expansions. Get-by-id is
limited to 10 posts, recent search to 25 posts, and creation to one post or one
reply. The console spending limit is the hard account-level control.

Before credits are purchased, the operator must confirm the exact initial
credit amount, billing-cycle spending limit, and any auto-recharge amount and
trigger. Before the live write smoke test, the operator must confirm the exact
text and destination account. Redacted handoff evidence may record app id,
callback, app type, secret version, budget, credit balance, and usage totals,
but never credential values.

Local component and runtime checks do not deploy the app. V1 has no test or
preproduction X app, callback, catalog, or Firna deployment. The reviewed
package is released only through the standard production workflow after it
merges to `main`, and live smoke validation runs in the nominated production
workspace against the intended production X account.

Current prices must be rechecked in the console immediately before purchase.
The public documentation references are [X API pricing], [OAuth 2.0 user access
tokens], [get posts by ids], [recent search], and [create post].

[X API pricing]: https://docs.x.com/x-api/getting-started/pricing
[OAuth 2.0 user access tokens]: https://docs.x.com/fundamentals/authentication/oauth-2-0/user-access-token
[get posts by ids]: https://docs.x.com/x-api/posts/get-posts-by-ids
[recent search]: https://docs.x.com/x-api/posts/search/introduction
[create post]: https://docs.x.com/x-api/posts/create-post
