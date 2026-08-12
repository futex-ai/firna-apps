# X Direct Messages Protocol

This document defines bounded Direct Message reads and writes for the
first-party [X app](x-app.md). DM data is private connection data and is shown
only to agents with access to that exact X installation.

## Read Direct Messages

`x_get_dms` requires `view` and accepts optional `max_results` 10-25 and
`pagination_token` for collection modes:

| View | Required selector | Provider endpoint |
| --- | --- | --- |
| `all` | none | `GET /2/dm_events` |
| `conversation` | `conversation_id` | `GET /2/dm_conversations/{id}/dm_events` |
| `participant` | `participant_id` | `GET /2/dm_conversations/with/{id}/dm_events` |
| `event` | `event_id` | `GET /2/dm_events/{id}` |

Single-event lookup rejects paging fields. Collection output contains ordered
`events`, an optional token, and `result_count`; single lookup contains one
event or returns `not_found`.

A compact event contains provider-returned `id`, `event_type`,
`dm_conversation_id`, `sender_id`, participant ids, text, and creation time.
The component requests no expansions, media downloads, or raw entities. Each
returned event reports one $0.010 `dm_event_read`, capped at 25.

## Manage Direct Messages

`x_manage_dm` accepts one closed `action`:

- `send_to_participant`: requires decimal `participant_id` and either non-blank
  `text` or one existing `media_id`; sends
  `POST /2/dm_conversations/with/{participant}/messages`.
- `send_to_conversation`: requires `conversation_id` and text or media id;
  sends `POST /2/dm_conversations/{conversation}/messages`.
- `create_group`: requires 2-10 unique decimal `participant_ids` and text or
  media id; sends `POST /2/dm_conversations` with type `Group` and one initial
  message.
- `delete`: requires decimal `event_id`; sends `DELETE /2/dm_events/{event}`.

Text is limited to 10,000 Unicode scalar values. Media ids reference objects
already uploaded to X; this app does not upload binary media. Message creation
reports $0.015 on provider-confirmed success and returns conversation and event
ids. Delete reports $0.010 and returns `deleted: true`.

Every write emits at most one provider request. A missing id, `deleted: false`,
malformed success, transport loss, truncation, missing status, or provider 5xx
after dispatch becomes `write_outcome_unknown`. No automatic retry, group
fan-out, or duplicate send is permitted.

## Bookmark Folders

`x_create_bookmark_folder` is kept separate from DM actions but uses the same
private-account safety rules. It requires decimal `user_id` and a trimmed name
of 1-25 characters, sends `POST /2/users/{id}/bookmarks/folders`, costs $0.005,
and returns the provider-confirmed folder id and name.

Folder reads and individual Post bookmark actions are defined by
[Posts and feeds](x-app-posts.md).

## Privacy and Errors

All DM tools require `dm.read` or `dm.write` in addition to the shared
`tweet.read` and `users.read` scopes required by X. A provider 403 identifies
the exact missing DM scope. Provider payload text is never copied into errors.

The app stores no DM content, conversation cache, read cursor, or background
subscription. Results live only in the normal bounded tool result and platform
history governed by workspace retention. Disconnecting one X account removes
that installation's future DM access without affecting other connections.
