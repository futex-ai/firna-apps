# HTTP App

`apps/http` is the repo-owned workspace-default app that exposes the
model-visible `http_request` tool. It replaces the old native `http` tool group
while keeping the same request and response contract for agents.

## Responsibilities

- Declare `http_request` as an installed app tool in the `apps` selector group.
- Use the first-party broad HTTP host capability to reach model-supplied URLs.
- Keep provider credentials unavailable to generic outbound HTTP requests.
- Default omitted request timeouts to 60 seconds and reject timeouts outside
  `1..=300` seconds at both the app and host boundaries.
- Normalize HTTP status, headers, content type, body, and truncation metadata
  for the model.
- Preserve parseable provider response headers except credential-bearing headers
  such as `authorization`, `cookie`, `proxy-authorization`, and `set-cookie`.

## Development

```bash
cargo test --manifest-path apps/http/component/Cargo.toml --locked
cargo test --manifest-path apps/http/tests/platform-runtime/Cargo.toml --locked
firna apps validate apps/http
firna apps package apps/http
```

Component builds and platform runtime tests honor `CARGO_TARGET_DIR`, allowing
Conductor workspaces to keep their build artifacts in workspace-specific
external target directories.

The app is intentionally built through the normal app package flow. Fresh tool
executions should persist `tool_group = "apps"` for `http_request`; legacy
history with `tool_group = "http"` remains display-only compatibility data.
