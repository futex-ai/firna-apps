# X Post Metrics Protocol

## Scope

Package version `2.0.1` retains `x_get_post_metrics` in the expanded first-party
X app. The tool reads a current metrics snapshot for explicitly selected Posts
through the standard Post lookup endpoint. Shared installation, OAuth,
host-credential, error, response-limit, and wallet behavior remains defined by
the [X app protocol](x-app.md).

This contract was checked against X's Post metrics, Post lookup, pricing, and
Enterprise documentation on 2026-08-08. Public pricing listed a Post read at
$0.005 per resource. The X Developer Console remains authoritative; release
must stop if its price or billing unit differs from this immutable package
contract.

## Tool Contract

The tool name is `x_get_post_metrics`, its operation is
`x.get_post_metrics`, and its side effect is `external_read`. It uses the
existing workspace-owned `x_workspace` authorization and `tweet.read` scope.
It does not add an OAuth scope or expose a credential to the component.

Input is a closed object:

- `ids`: required array of 1-10 unique decimal Post-id strings matching
  `^[0-9]{1,19}$`
- `include_private_metrics`: optional boolean, default `false`

Unknown properties, JSON numbers in place of ids, duplicate ids, and arrays
outside the bound produce `invalid_request` with `invalid_post_ids`. Invalid
input sends no provider request and reports no usage.

## Provider Request

Each invocation sends at most one request:

```text
GET https://api.x.com/2/tweets
ids=<ids joined in caller order>
tweet.fields=public_metrics
```

When `include_private_metrics` is true, `tweet.fields` is exactly
`public_metrics,non_public_metrics`. The component does not request author,
organic, promoted, media, or Ads fields. It does not paginate, retry, poll, or
persist snapshots. The host may perform only the single authentication-refresh
retry defined by the shared protocol.

## Typed Metric Mapping

Every returned metrics item has the provider-confirmed `id` and a required
`public_metrics` object. Provider fields map to output fields as follows:

| Provider field | Output field |
| --- | --- |
| `public_metrics.impression_count` | `public_metrics.impressions` |
| `public_metrics.like_count` | `public_metrics.likes` |
| `public_metrics.reply_count` | `public_metrics.replies` |
| `public_metrics.retweet_count` | `public_metrics.reposts` |
| `public_metrics.quote_count` | `public_metrics.quotes` |
| `public_metrics.bookmark_count` | `public_metrics.bookmarks` |
| `non_public_metrics.engagements` | `private_metrics.engagements` |
| `non_public_metrics.url_link_clicks` | `private_metrics.url_clicks` |
| `non_public_metrics.user_profile_clicks` | `private_metrics.profile_clicks` |

Counts are non-negative JSON integers. A present zero remains `0`. Missing,
null, negative, fractional, overflowing, or otherwise malformed required
public counts make the entire call a redacted `provider_contract_error`.
Unknown provider properties are discarded.

Private fields are requested only when the caller opts in. For each returned
Post, `private_metrics` contains only the documented private counts X returns.
An `unavailable_private_metrics` array names omitted fields in the fixed order
`engagements`, `url_clicks`, `profile_clicks`. The array is omitted when all
three fields are available, and both private fields are omitted when private
metrics were not requested. A malformed present private count is a provider
contract error; an absent field is an unavailable value, never zero.

X documents private metrics only for Posts owned by the authorizing user and
created within the last 30 days. The output does not guess whether ownership,
age, authorization policy, or another provider rule caused an omission.
`profile_clicks` means clicks from that Post to its author's profile. It is not
an account's total profile views and must never be labelled `profile_views`.

## Output and Partial Results

A successful output has:

- `metrics`: returned Post metrics ordered by the caller's `ids`, regardless of
  provider response order
- `missing_ids`: requested ids X omitted, in caller order; omitted when empty
- `result_count`: the number of entries in `metrics`

The component rejects duplicate provider ids or provider ids not requested by
the caller as `provider_contract_error`. A partial provider result succeeds and
lists missing ids. If X returns none of the requested Posts, the tool returns
`not_found` and reports no usage.

Provider text, default Post fields, provider error bodies, request signatures,
credentials, account ids, and billing identifiers never appear in tool output
or handled errors.

## Errors and Resource Bounds

The shared read-error mapping applies:

- missing or terminal authorization becomes `auth_required`
- HTTP 403 becomes `missing_scope` for `tweet.read`
- HTTP 404 or a completely missing result becomes `not_found`
- HTTP 429 becomes `rate_limited`, with a safe parsed retry delay when present
- X usage exhaustion becomes `provider_budget_exhausted`
- other provider 4xx responses become `invalid_request` with
  `provider_rejected_request`
- transport failures and HTTP 5xx responses become `provider_unavailable`
- missing, malformed, or truncated successful bodies become
  `provider_contract_error`

The manifest and component retain the 262,144-byte response limit and 30-second
component/provider timeout. Every handled failure is uncharged.

## Billing

`x_get_post_metrics` reports only the existing metered `post_read` unit:

- price: 5,000 micro-USD ($0.005) per validated returned Post
- maximum units: 10 per call
- maximum wallet hold: 50,000 micro-USD ($0.050)

Missing ids and failed calls add no units. Private fields do not add a second
unit because they are fields on the same returned Post resource. Usage is
created only after the complete provider response and metric objects validate,
then the platform strips usage before exposing output to the agent. If the
Developer Console lists a different price or resource unit, operators must not
release or invoke this version; the protocol, manifest, tests, and package
version must change together before a new consented release.

## Non-goals

This tool does not provide total account profile views, time-series or
historical analytics, scheduled collection, media/video analytics, organic or
promoted breakdowns, Ads API reporting, or Enterprise engagement endpoints.
In particular, it does not call `GET /2/tweets/analytics`,
`GET /2/media/analytics`, or `ads-api.x.com`.

Official references are [X metrics], [Post lookup], [X API pricing], and
[Enterprise API access].

[X metrics]: https://docs.x.com/x-api/fundamentals/metrics
[Post lookup]: https://docs.x.com/x-api/posts/get-posts-by-ids
[X API pricing]: https://docs.x.com/x-api/getting-started/pricing
[Enterprise API access]: https://docs.x.com/enterprise-api/getting-started/about-x-api
