# GitHub Component

This crate builds the Wasm component for Firna's repo-owned GitHub App. Depend
on it only through the app package runtime ABI; installation verification,
token minting, workspace routing, durable lifecycle changes, and event
deduplication remain platform responsibilities.

## Responsibilities

- Decode and validate calls for the five GitHub read tools.
- Construct bounded, known `api.github.com` GET requests.
- Request host-mediated bearer injection without receiving the raw token.
- Verify GitHub HMAC signatures through an opaque host secret handle.
- Classify ping, installation lifecycle, and six deliverable event types.
- Convert provider responses and webhook payloads into bounded, redacted
  projections.

## What This Crate Does

The component exports `call-tool`, `verify-webhook`, `webhook-response`,
and `normalize-event`. Tool code accepts no arbitrary URL input, rejects
host-truncated responses, and maps provider failures to stable errors. A host
`credential_not_found` response becomes `auth_required` for
`github_installation`; provider, vault, network, malformed, and unknown host
failures remain provider-unavailable.

Exact file reads accept at most 16 path segments, resolve the requested ref to
a commit, and walk non-recursive Git trees one segment at a time. Only regular
and executable blobs are accepted; directories, symlinks, and submodules are
rejected before the final commit-pinned Contents request.

Webhook verification uses the exact raw UTF-8 body, requires one lower-case
`sha256=` digest, and compares it in constant time. It validates the delivery
and event headers plus event-specific payload shape before returning trusted
installation, account, actor, and lifecycle metadata. Ping returns
`{"ok":true}`; only the six manifest-declared content events normalize.

Normalization caps commit lists at 20, titles and names at 256 characters,
commit messages at 512, and bodies and comments at 2,000. It retains only
canonical GitHub URLs and typed identity or summary fields. Unknown fields,
signatures, secrets, tokens, provider errors, and webhook patch text never enter
normalized output.

## Quick Start

```sh
cargo test --manifest-path apps/github/component/Cargo.toml --locked
cargo build --manifest-path apps/github/component/Cargo.toml \
  --target wasm32-unknown-unknown --locked
```

## Development

Unit tests mock `GitHubProvider`, `Clock`, and `WebhookSigner` with
`unimock`. Fixtures cover every subscribed event and implicit control shape.

### Key Code

- `src/lib.rs`: WIT imports and all four component exports.
- `src/github/tools/`: request construction and tool projections.
- `src/github/webhook_host.rs`: opaque HMAC host requests.
- `src/github/webhook_validation.rs`: authentication and classification.
- `src/github/webhook_projection.rs`: model-visible event projection.

### Related Docs

- [GitHub package](../README.md)
- [GitHub app protocol](../../../docs/protocol/github-app.md)
- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
