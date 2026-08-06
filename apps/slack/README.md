# Slack App

`apps/slack` is the repo-owned Slack app package. Its component is a standalone
Cargo workspace, so platform crates cannot depend on Slack source directly.

## Layout

- `manifest.yaml`: preferred app manifest, public env values, and required
  secret names.
- `component/`: Rust guest source and `Cargo.lock` that Firna servers build
  into the production Wasm component.
- `assets/`: app-owned icon source and rendered catalog icon.
- `tests/platform-runtime/`: standalone app-side tests that build the component
  source and exercise it through the platform Wasm host.
- Slack message sends pass the durable tool `operation_id` through as
  `chat.postMessage.client_msg_id`, so resumed tool execution reuses the same
  provider-facing idempotency key.

## Local Commands

```bash
firna apps validate apps/slack
firna apps package apps/slack
rustup target add wasm32-unknown-unknown
cargo build --manifest-path apps/slack/component/Cargo.toml --target wasm32-unknown-unknown --locked
cargo test --manifest-path apps/slack/tests/platform-runtime/Cargo.toml --locked
```

Component builds and platform runtime tests honor `CARGO_TARGET_DIR`, allowing
Conductor workspaces to keep their build artifacts in workspace-specific
external target directories.

Production deployment uploads source only. The trusted Firna build service
creates and validates the Wasm component during admin submit before the version
can be promoted live.

## Tools

| App | Tool | Inputs | Description | Authentication |
| --- | --- | --- | --- | --- |
| slack | `slack_list_channels` | `cursor?`, `exclude_archived?`, `limit?`, `types?` | List Slack channels visible to the workspace app bot. | Active Slack install with `slack_bot` workspace auth. |
| slack | `slack_read_channel_history` | `channel_id`, `cursor?`, `oldest?`, `latest?`, `limit?` | Read recent Slack channel messages visible to the app bot. | Active Slack install with `slack_bot` workspace auth. |
| slack | `slack_send_message` | `channel_id`, `text`, `thread_ts?` | Send a Slack message as the workspace app bot. | Active Slack install with `slack_bot` workspace auth. |
| slack | `slack_search_messages` | `query`, `cursor?`, `limit?`, `sort?`, `sort_dir?` | Search Slack messages using a user-authorized Slack grant. | Active Slack install with `slack_bot` workspace auth and `slack_user_search` user grant. |

## Events

Slack exposes `app_mention`, `message_channels`, `message_groups`,
`message_im`, and `message_mpim` as stable native events. Installing Slack
does not choose a handler or wake an agent automatically. Each agent explicitly
subscribes to the events it needs after the workspace installation is active.
The manifest nests these events under `slack_events` and forwards only
`x-slack-request-timestamp` and `x-slack-signature` to its verifier.

## Secrets

The manifest declares:

- `client_secret`
- `signing_secret`

Production CI resolves those from Google Secret Manager as
`firna-prod-app-slack-client-secret` and
`firna-prod-app-slack-signing-secret`, then passes the values separately to
`firna admin apps submit`.
