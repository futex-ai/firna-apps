# DataForSEO

DataForSEO is a built-in, explicitly installed Firna app for bounded search,
keyword, backlink, page, business, content, domain, and AI visibility research.
It uses the customer's own DataForSEO API login and generated password.

## Installation

A workspace owner or admin installs the app from **Settings → Installed apps**,
enters both values from DataForSEO API Access, and confirms that provider calls
are billed to the workspace's DataForSEO account. Firna verifies the pair with
the free API Status endpoint, encrypts it, and never prefills it later. The app
declares a 64 KiB credential-verification response budget for the small status
response, so verification never depends on the generic host HTTP fallback. No
account, usage, balance, or pricing data is requested.

No Firna-owned DataForSEO account, deployment secret, or runtime environment
variable is required. Configure spending limits in DataForSEO before enabling
agents.

## Tools

The package exposes 16 synchronous, read-only tools. Every invocation submits
one bounded Live task, returns compact normalized records with task cost and
rate-limit metadata, and never creates a polling handle or deferred operation.
See the [tool protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/dataforseo-app-tools.md)
for the complete schemas.
The model-visible LLM Mentions schema exposes ChatGPT's US/English selector
restriction, and the component enforces the same rule before provider work.

## Development

```sh
cargo test --manifest-path apps/dataforseo/component/Cargo.toml --locked
cargo test --manifest-path apps/dataforseo/tests/platform-runtime/Cargo.toml --locked
```

The component and runtime tests use a fake Firna host. Live DataForSEO
credentials are never required by automated tests.

## Related Docs

- [DataForSEO app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/dataforseo-app.md)
- [Synchronous tool schemas](https://github.com/futex-ai/firna/blob/main/docs/protocol/dataforseo-app-tools.md)
- [App packages](../README.md)
