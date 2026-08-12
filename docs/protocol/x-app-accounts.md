# X Accounts and Relationships Protocol

This document defines account lookup and social-relationship tools for the
first-party [X app](x-app.md).

## Account Profiles

A compact account profile contains required `id`, `name`, and `username`, plus
provider-returned `description`, `created_at`, `location`, `url`,
`profile_image_url`, `protected`, `verified`, `verified_type`, and public
follower/following/Post/List counts. Missing fields are omitted.

`x_get_users` accepts `lookup` with exactly one compatible selector:

- `me`: no ids; sends `GET /2/users/me`
- `ids`: 1-10 unique decimal `ids`; sends `GET /2/users?ids=...`
- `usernames`: 1-10 unique `usernames`, each 1-50 characters matching X's
  handle alphabet; sends `GET /2/users/by?usernames=...`

All modes request the compact profile fields. Output contains `users`, optional
`missing_values` in request order, and `result_count`. A `me` response or a
batch with no returned account is `not_found`. Usage is one `user_read` per
returned account, capped at 10.

`x_search_users` accepts a query of 1-50 ASCII letters, digits, apostrophes,
underscores, or spaces; `max_results` 10-25; and optional
`pagination_token`. It sends `GET /2/users/search`, returns `users`, optional
token, and `result_count`, and reports up to 25 User reads.

## Relationship Reads

`x_get_relationships` requires decimal `user_id`, `relationship`,
`max_results` 10-25, and optional `pagination_token`. Modes are:

| Relationship | Provider endpoint | Scope |
| --- | --- | --- |
| `affiliates` | `GET /2/users/{id}/affiliates` | `users.read` |
| `followers` | `GET /2/users/{id}/followers` | `follows.read` |
| `following` | `GET /2/users/{id}/following` | `follows.read` |
| `blocked` | `GET /2/users/{id}/blocking` | `block.read` |
| `muted` | `GET /2/users/{id}/muting` | `mute.read` |

Output contains compact `users`, optional token, and `result_count`. Each
returned relationship reports one $0.010 `user_read`, capped at 25.

## Relationship Actions

`x_manage_relationship` requires decimal `user_id`, `target_user_id`, and one
closed action:

| Action | Provider request | Scope | Cost |
| --- | --- | --- | ---: |
| `follow` | `POST /2/users/{user}/following` | `follows.write` | $0.015 |
| `unfollow` | `DELETE /2/users/{user}/following/{target}` | `follows.write` | $0.010 |
| `mute` | `POST /2/users/{user}/muting` | `mute.write` | $0.015 |
| `unmute` | `DELETE /2/users/{user}/muting/{target}` | `mute.write` | $0.005 |
| `dm_block` | `POST /2/users/{target}/dm/block` | `dm.write` | $0.010 |
| `dm_unblock` | `POST /2/users/{target}/dm/unblock` | `dm.write` | $0.010 |

POST follow/mute bodies contain only `target_user_id`. DM block routes identify
the target in the path; `user_id` still binds the caller's explicit account
choice and must be a valid decimal id. Output contains the action, target id,
and provider-confirmed boolean state. Contradictory or missing confirmation is
`write_outcome_unknown`; one invocation sends at most one request.

## Validation and Authorization

All ids are 1-19 decimal characters. Batches reject duplicates. Usernames are
passed without `@`. Mode-incompatible selectors, blank pagination tokens, and
unknown enum values fail before provider dispatch.

A 403 maps to the exact mode or action scope listed above, or `users.read` for
profile tools. The host may refresh and replay one identical request after a
401; the component itself never retries. Profiles and relationship results do
not expose provider-only connection status, private email, or raw entities.
