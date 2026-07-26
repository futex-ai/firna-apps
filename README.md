# Firna Apps

First-party app packages maintained by Firna. This repository keeps trusted
Firna-owned apps separate from the platform repository and from independently
published community apps.

The current catalog packages are:

- DataForSEO: explicitly installed research tools backed by each workspace's
  own DataForSEO credentials.
- Exa: workspace-default web search backed by a Firna-managed API key.
- HTTP: workspace-default, first-party arbitrary-host HTTP requests.
- Slack: explicitly installed Slack tools, OAuth, webhooks, and event handling.

Each package is an isolated Rust WebAssembly component under
[`apps/`](apps/README.md). Production deployment uploads source bundles to the
Firna app builder; developer-built Wasm files are never production artifacts.

## Development

Install the toolchain and compatibility tools recorded in
[`platform.toml`](platform.toml), then run the complete repository verifier:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-tools --locked --version 1.248.0
cargo xtask check
```

The app platform integration tests use `fna-apps-interface` and
`fna-apps-wasm` directly from the pinned Firna platform revision. Update the
revision in `platform.toml` and every standalone runtime test manifest together;
`cargo xtask check` rejects partial updates.

To install the matching `firna` CLI for local package validation:

```sh
cargo install --locked \
  --git https://github.com/futex-ai/firna.git \
  --rev 825dffab745c402db8c38501d73d05548a4f238d \
  --bin firna fna-cli
firna apps validate apps/slack
```

See [`apps/README.md`](apps/README.md) for package commands and conventions.

## Deployment

Pull requests and `main` pushes run [CI](.github/workflows/ci.yml). After a
successful CI run on `main`, [Deploy Firna Apps](.github/workflows/deploy-apps.yml)
compares every local manifest with the live catalog and submits only missing or
newer versions. A manual dispatch can force one app or all apps to be
resubmitted.

Deployment authenticates as the production global admin and uses
`firna admin apps submit`. That operator-controlled route builds, approves, and
promotes these trusted packages in one operation; it deliberately does not use
the community submission/review path. Required app values are read from Google
Secret Manager and passed by environment variable without entering manifests,
source bundles, or logs.

The workflow expects these GitHub Actions variables:

- `GCP_SERVICE_ACCOUNT`
- `GCP_WORKLOAD_IDENTITY_PROVIDER`
- `FIRNA_BOOTSTRAP_USERNAME`

CI and deployment also require the repository Actions secret
`FIRNA_REPOSITORY_TOKEN`. It should contain a machine token restricted to
reading the private `futex-ai/firna` platform repository. Google Cloud IAM does
not grant access to private GitHub source dependencies.

The Google identity should be dedicated to this repository and limited to
reading the production bootstrap password plus manifest-declared app secrets.

## Key Code

- [`apps/`](apps/README.md): manifests, component source, assets, and runtime
  tests.
- [`scripts/repository_audit.py`](scripts/repository_audit.py): compatibility,
  version, source-layout, and file-length checks.
- [`scripts/plan-app-deploys.py`](scripts/plan-app-deploys.py): catalog-aware
  production deployment planning.
- [`xtask/`](xtask/README.md): local and CI verification entrypoints.

The platform-side manifest, runtime, and admin submission contracts remain in
the [Firna platform app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md).
