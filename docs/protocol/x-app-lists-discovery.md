# X Lists and Discovery Protocol

This document defines Lists, Spaces, Communities, trends, and media-metadata
tools for the first-party [X app](x-app.md).

## Lists

`x_get_lists` requires `view`, optional `max_results` 10-25 and
`pagination_token`, plus the selector shown below:

| View | Selector | Provider endpoint | Result |
| --- | --- | --- | --- |
| `list` | `list_id` | `GET /2/lists/{id}` | Lists |
| `owned` | `user_id` | `GET /2/users/{id}/owned_lists` | Lists |
| `followed` | `user_id` | `GET /2/users/{id}/followed_lists` | Lists |
| `memberships` | `user_id` | `GET /2/users/{id}/list_memberships` | Lists |
| `pinned` | `user_id` | `GET /2/users/{id}/pinned_lists` | Lists |
| `posts` | `list_id` | `GET /2/lists/{id}/tweets` | Posts |
| `members` | `list_id` | `GET /2/lists/{id}/members` | Users |
| `followers` | `list_id` | `GET /2/lists/{id}/followers` | Users |

Single-List and pinned-List lookups reject paging fields. X's pinned-List
endpoint does not accept paging parameters, so the app requests its
provider-bounded set and rejects responses above the manifest's 25-List billing
ceiling. Output includes only the applicable `lists`, `posts`, or `users`,
optional Post `authors`, an optional token, and `result_count`. List reads cost
$0.005 each; Post reads $0.005; User reads $0.010. Every collection is capped
at 25.

`x_manage_list` uses one closed action:

- `create`: requires `name` (1-25 characters); optional `description` (100) and
  `private`; sends `POST /2/lists`; costs $0.010.
- `update`: requires `list_id` and at least one mutable field; sends
  `PUT /2/lists/{id}`; costs $0.005.
- `delete`: requires `list_id`; sends `DELETE /2/lists/{id}`; costs $0.005.
- `add_member`/`remove_member`: require `list_id` and `target_user_id`; send the
  relevant member POST/DELETE; cost $0.005.
- `follow`/`unfollow`/`pin`/`unpin`: require `user_id` and `list_id`; send the
  relevant user-List request; cost $0.005.

Create/update returns the compact provider List. Other actions return the
action, List id, optional target id, and provider-confirmed boolean state.

## Spaces

`x_get_spaces` accepts `view` and mode-specific selectors:

- `ids`: 1-10 `ids`; `GET /2/spaces`
- `creators`: 1-10 `creator_ids`; `GET /2/spaces/by/creator_ids`
- `search`: `query` up to 2,048 characters, optional `state` (`live`,
  `scheduled`, `all`), and `max_results` 10-25; `GET /2/spaces/search`
- `posts`: `space_id`, `max_results`, optional token; `GET /2/spaces/{id}/tweets`
- `buyers`: `space_id`, `max_results`, optional token; `GET /2/spaces/{id}/buyers`

Space output includes `id`, state, title, creator, language, participant count,
ticketing flag, and available schedule/start/end times. Mode output contains
`spaces`, `posts`, or `users`, optional token, and `result_count`. Usage reports
the returned Space, Post, and User resource categories, each capped at 25.

## Communities

`x_get_communities` accepts `view`:

- `ids`: 1-10 unique decimal `ids`; sends one `GET /2/communities/{id}` only
  when exactly one id is supplied. Multiple ids are rejected because X exposes
  no batch endpoint.
- `search`: non-blank `query` up to 4,096 characters, `max_results` 10-25, and
  optional `pagination_token`; sends `GET /2/communities/search`.

Output communities contain id, name, description, access, join policy, member
count, and creation time when returned. Each resource reports one $0.005
`community_read`.

## Trends

`x_get_trends` accepts `view`:

- `personalized`: no selector; `GET /2/users/personalized_trends` with the
  connection token; X controls the page and the manifest caps 50 trends.
- `location`: required positive 32-bit `woeid` and optional `max_trends` 1-25;
  `GET /2/trends/by/woeid/{woeid}` with the app bearer.

Output contains trend name and optional Post count. Every returned trend costs
$0.010, capped at 50 for personalized and 25 for location.

## Media Metadata

`x_get_media` accepts 1-10 unique `media_keys` matching
`^[0-9]+_[0-9]+$` and sends `GET /2/media`. Output contains key, type,
duration, dimensions, preview URL, and public metrics when X returns them.
Each returned media resource costs $0.005. Raw media bytes and expiring variant
URLs are not downloaded or exposed as workspace files.

`x_manage_media` requires `media_id` and one closed action:

- `set_alt_text` requires 1-1,000 characters and sends one typed
  `POST /2/media/metadata` body.
- `add_subtitles` requires an existing subtitle `subtitle_media_id`, a display
  name up to 100 characters, a two-letter uppercase `language_code`, and
  `media_category` (`AmplifyVideo` or `TweetVideo`); it sends one
  `POST /2/media/subtitles`.
- `delete_subtitles` requires the language and category and sends one
  `DELETE /2/media/subtitles` with a JSON body.

Each action requires `media.write`, costs $0.005, and returns the action,
target media id, and a provider-confirmed `applied: true`. Missing or
contradictory confirmation is `write_outcome_unknown`. These actions manage
already-uploaded objects; they do not upload binary media.
