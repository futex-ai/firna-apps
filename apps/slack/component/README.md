# fna-app-slack-component

`fna-app-slack-component` is the Rust guest source for the Slack app Wasm
component. It lives under `apps/slack` as a standalone Cargo workspace.

## Responsibilities

- Implement the `firna-app-component-v1` guest exports for Slack.
- Use host imports for Slack HTTP calls, HMAC webhook verification, and
  redacted logs.
- Keep Slack provider DTO handling inside the component boundary.

## What This Crate Does

The crate exports `call-tool`, `verify-webhook`, `webhook-response`, and
`normalize-event` through `wit-bindgen`. The Firna app build service compiles the
source to `wasm32-unknown-unknown`, wraps it as a Wasm component, validates the
ABI, stores the artifact, and waits for approval before promotion.
For `slack_send_message`, the component forwards the host-provided
`operation_id` as Slack `chat.postMessage.client_msg_id`, giving durable agent
tool execution a provider-facing idempotency key.

## Quick Start

```bash
rustup target add wasm32-unknown-unknown
cargo build --manifest-path apps/slack/component/Cargo.toml --target wasm32-unknown-unknown --locked
```

## Development

```bash
cargo build --manifest-path apps/slack/component/Cargo.toml --target wasm32-unknown-unknown --locked
firna apps validate apps/slack
```

### Key Code

- `src/lib.rs`: WIT world declaration and exported component entrypoints.
- `src/slack/tools.rs`: Slack Web API tool handlers.
- `src/slack/webhooks.rs`: Slack signature verification and event
  normalization.
- `src/slack/host.rs`: JSON helpers for Firna host imports.

### Related Docs

- [Package README](../README.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [Firna app deployment](https://github.com/futex-ai/firna/blob/main/docs/deployment/apps.md)
