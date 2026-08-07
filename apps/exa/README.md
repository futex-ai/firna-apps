# Exa App

`apps/exa` is the built-in Exa web-search app package. It exposes the
`exa_web_search` tool through the Firna app runtime instead of the native agent
tool surface, with an optional workspace-owned Exa API key.

## Responsibilities

- Declare the Exa app manifest, default install policy, allowed provider host,
  optional workspace API-key flow, and required app-owned fallback key.
- Build the Wasm component that maps model-visible search input to Exa's
  `/search` request shape.
- Keep both workspace and fallback Exa keys opaque to component memory by
  requesting host-mediated `x-api-key` injection.

## What This App Does

The package installs by default when it is present in the app catalog. Firna's
managed key provides zero-configuration search; a workspace admin can add,
replace, or remove a workspace-owned Exa key in the app's Settings page. The
workspace key takes precedence immediately, and removing it restores the
managed fallback. Exa validates the key on its first search request; if Exa
rejects it, update or remove the key in Settings.

The component accepts `exa_web_search` requests, validates limits locally,
calls `https://api.exa.ai/search`, and returns the provider JSON with Firna
`provider`, `status`, and `ok` metadata.

## Quick Start

```bash
firna apps validate apps/exa
firna apps package apps/exa
cargo build --manifest-path apps/exa/component/Cargo.toml --target wasm32-unknown-unknown --locked
cargo test --manifest-path apps/exa/tests/platform-runtime/Cargo.toml --locked
```

Component builds and platform runtime tests honor `CARGO_TARGET_DIR`, allowing
Conductor workspaces to keep their build artifacts in workspace-specific
external target directories.

Local submit requires a running server, an admin login, and an app secret:

```bash
firna config set-server http://127.0.0.1:50051
firna admin login --email dev
firna admin apps submit apps/exa \
  --secret-env api_key=EXA_API_KEY
```

## Development

### Key Code

- `manifest.yaml` declares the `workspace_default` app, `api.exa.ai` HTTP
  capability, optional workspace API-key flow, app-owned fallback,
  `x-api-key` credential header, and `exa_web_search` schema.
- `component/src/exa/tools.rs` owns request validation and Exa request
  rendering.
- `component/src/exa/host.rs` owns host HTTP calls and error normalization.
- `tests/platform-runtime` validates package metadata and host-mediated tool
  execution against a fake provider response.

### Related Docs

- [Firna app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md)
- [Firna app API-key flow](https://github.com/futex-ai/firna/blob/main/docs/protocol/app-api-key-flow.md)
- [Firna agent-tool protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/agent-tools.md)
- [Firna app deployment](https://github.com/futex-ai/firna/blob/main/docs/deployment/apps.md)
