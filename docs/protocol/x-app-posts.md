# X Posts and Feeds Protocol

This document defines Post, timeline, engagement, count, and Post-action tools
for the first-party [X app](x-app.md). All collections are one bounded page.

## Shared Post Reads

`include_authors` defaults to `false`. When true, the component requests
`expansions=author_id` and compact User fields and reports both `post_read` and
`user_read`; otherwise it requests and reports no author resources.

`x_get_posts` accepts 1-10 unique `ids` and optional `include_authors`. It sends
one `GET /2/tweets`, preserves requested order in `missing_ids`, and returns
`not_found` only when no requested Post is returned.

`x_get_post_metrics` retains the separate
[metrics contract](x-app-metrics.md): 1-10 ids, optional private owned-Post
metrics, one lookup, and only `post_read` usage.

## Post Search

`x_search_recent_posts` retains its installed contract: non-blank `query` up to
512 characters, required `max_results` 10-25, optional `next_token`, and
optional `include_authors`. It sends `GET /2/tweets/search/recent` and returns
`posts`, optional `authors` and `next_token`, and `result_count`.

All Post search and count tools accept X's web-search engagement aliases for
compatibility. Outside quoted phrases, the component translates `min_faves:`
to the X API v2 `min_likes:` operator and `min_retweets:` to `min_reposts:`.
Already-canonical operators and alias text inside quoted phrases are unchanged.

`x_search_all_posts` accepts:

- `query`: 1-4,096 characters
- `max_results`: 10-25
- optional `pagination_token`, `start_time`, `end_time`, and `include_authors`

It sends `GET /2/tweets/search/all` with the app-owned bearer token. Time values
are non-blank strings of at most 64 bytes and are provider-validated. The
result shape matches recent search but uses `pagination_token`. It reports up
to 25 Post and 25 User reads.

## Post Counts

`x_get_post_counts` accepts `range` (`recent` or `all`), `query`, `granularity`
(`minute`, `hour`, or `day`), optional start/end time, and optional
`pagination_token`. Recent query text is capped at 512 characters and rejects a
pagination token; all-history text is capped at 4,096 characters.

The component sends one `GET /2/tweets/counts/recent` or
`GET /2/tweets/counts/all` with the app bearer. Output contains ordered
`buckets` (`start`, `end`, `post_count`), `total_post_count`, and an optional
all-history `pagination_token`. Successful usage is $0.005 for recent or $0.010
for all, including an empty count result.

## User Feeds

`x_get_user_feed` accepts `feed`, required `max_results` 10-25, optional
`user_id`, optional `pagination_token`, and optional `include_authors`. For
every mode except `reposts_of_me`, an omitted `user_id` targets the selected
connection by first sending `GET /2/users/me`, validating its decimal id, and
then sending the mode's page request. An explicit id skips that lookup. Modes
are:

| Mode | Extra required input | Provider endpoint |
| --- | --- | --- |
| `posts` | none | `GET /2/users/{id}/tweets` |
| `mentions` | none | `GET /2/users/{id}/mentions` |
| `home` | none | `GET /2/users/{id}/timelines/reverse_chronological` |
| `liked` | none | `GET /2/users/{id}/liked_tweets` |
| `bookmarks` | none | `GET /2/users/{id}/bookmarks` |
| `bookmark_folder` | `folder_id` | `GET /2/users/{id}/bookmarks/folders/{folder}` |
| `bookmark_folders` | none | `GET /2/users/{id}/bookmarks/folders` |
| `reposts_of_me` | none | `GET /2/users/reposts_of_me` |

`exclude_replies` and `exclude_reposts` are accepted only for `posts` or
`mentions` and map to X's `exclude` query. Bookmark-folder mode returns
`bookmark_folders`; every other mode returns `posts`, optional `authors`, an
optional token, and `result_count`. Post modes report up to 25 Post reads and
25 expanded-author User reads. Resolving an omitted user id adds one User read,
for a maximum of 26. Folder listing reports no provider-billed Post resource
and reports only the optional identity lookup.

## Post Engagement

`x_get_post_engagements` requires `post_id`, `view`, `max_results` 10-25,
optional `pagination_token`, and optional `include_authors`. Modes are:

- `quotes`: `GET /2/tweets/{id}/quote_tweets`, returning Posts
- `reposts`: `GET /2/tweets/{id}/retweets`, returning Posts
- `liking_users`: `GET /2/tweets/{id}/liking_users`, returning Users
- `reposting_users`: `GET /2/tweets/{id}/retweeted_by`, returning Users

Output includes only the applicable `posts` or `users`, optional expanded
authors for Post modes, a token, and `result_count`. It reports up to 25 Post
and 25 User reads.

## Create or Edit a Post

`x_create_post` requires non-blank `text` up to 280 characters and accepts:

- optional `reply_to_post_id`, `quote_post_id`, or `edit_post_id`; at most one
- optional `poll_options` (2-4 entries, each 1-25 characters) paired with
  `poll_duration_minutes` (5-10,080)
- optional `media_ids` (1-4 existing ids), `community_id`, `reply_settings`,
  `made_with_ai`, and `paid_partnership`
- optional `allow_link`, default `false`

`reply_settings` is `following`, `mentioned_users`, `subscribers`, or
`verified`; the component maps `mentioned_users` to X's `mentionedUsers`.
Polls cannot be combined with media, reply, quote, or edit. Link-bearing text
requires `allow_link: true`. The component sends one `POST /2/tweets` and
returns the provider-confirmed compact Post. Success reports $0.015, or $0.200
when its bounded link detector finds `http://` or `https://`.

## Manage a Post

`x_manage_post` requires `action` and `post_id`. `user_id` is additionally
required for `repost`, `unrepost`, `like`, `unlike`, `bookmark`, and
`unbookmark`. Actions map to one request:

| Action | Request | Successful cost |
| --- | --- | ---: |
| `delete` | `DELETE /2/tweets/{post}` | $0.005 |
| `repost` / `like` | relevant `POST /2/users/{user}/...` | $0.015 |
| `unrepost` / `unlike` | relevant `DELETE /2/users/{user}/...` | $0.010 |
| `bookmark` / `unbookmark` | relevant bookmark POST/DELETE | $0.005 |
| `hide_reply` / `unhide_reply` | `PUT /2/tweets/{post}/hidden` | $0.010 |

The output contains the selected action, `post_id`, and the provider-confirmed
boolean state. A missing or contradictory confirmation is
`write_outcome_unknown`; the component never retries.
