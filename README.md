# Firna Apps

First-party app packages maintained by Firna. This repository keeps trusted
Firna-owned apps separate from the platform repository and from independently
published community apps.

The current catalog packages are:

- DataForSEO: explicitly installed research tools backed by each workspace's
  own DataForSEO credentials.
- Exa: workspace-default web search with an optional workspace-owned Exa API
  key and a Firna-managed fallback.
- GitHub: explicit workspace installation for short-lived external-repository
  credentials, five bounded read tools, and signed repository events.
- HTTP: workspace-default, first-party arbitrary-host HTTP requests.
- Slack: explicitly installed Slack tools, OAuth, webhooks, and event handling.
- X: explicitly installed, multi-account workspace-authorized, usage-priced
  Post reads, recent search, and single-Post publishing.

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
`fna-apps-wasm` directly from the pinned Firna platform revision, and the
deployment workflow installs its CLI from the same revision. Update
`platform.toml`, every standalone runtime test manifest, and the deployment
workflow together; `cargo xtask check` rejects partial updates.

To install the matching `firna` CLI for local package validation:

```sh
cargo install --locked \
  --git https://github.com/futex-ai/firna.git \
  --rev 733d089519f799b78f52a173db5cc1507fd72e65 \
  --bin firna fna-cli
firna apps validate apps/slack
```

The GitHub manifest uses the platform's installation-token flow; the X manifest
uses its usage-based app pricing and OAuth refresh lifecycle. Their validation,
packaging, and runtime tests are part of the canonical repository checks.

CI also runs a secret-provisioning merge gate
(`scripts/check_app_secrets.py`): for every environment class an app targets
(root and per-app [`deploy.toml`](deploy.toml)), each manifest-declared
secret must have an enabled value in the `firna-apps` Google Cloud project.
The gate creates missing containers itself and fails the pull request with
the exact `gcloud secrets versions add` command for every missing value, so
provisioning happens before merge, never after.

See [`apps/README.md`](apps/README.md) for package commands and conventions.
Active and completed implementation work is tracked in
[`plans/README.md`](plans/README.md).

## Deployment

Pull requests and `main` pushes run [CI](.github/workflows/ci.yml). After a
successful CI run on `main`, [Deploy Firna Apps](.github/workflows/deploy-apps.yml)
deploys every environment instance declared in [`deploy.toml`](deploy.toml) —
currently `production` and the stable `br-main` preview. Per instance it
compares every targeted local manifest with that instance's live catalog and
submits only missing or newer versions. The workflow also runs on a daily
schedule (so a reset environment converges without coordination), on the
`firna-platform-deployed` repository dispatch the platform deploy can send,
and on manual dispatch with optional `app` and `instance` inputs to force
resubmission.

Apps target environment classes through per-app `deploy.toml` files. `github`
and `x` deploy to production and the stable `br-main` preview, using separate
provider registrations whose fixed callbacks are registered for each long-lived
environment. Ephemeral `pr-N` previews exclude both apps. Deployment
authenticates per instance as that
instance's admin and uses `firna admin apps submit`. That operator-controlled
route builds, approves, and promotes these trusted packages in one operation;
it deliberately does not use the community submission/review path. App secret
values are read from the dedicated `firna-apps` Google Cloud project
(`<secret_prefix>-<app_id>-<secret-name-kebab>`) and passed by environment
variable without entering manifests, source bundles, or logs. Only the two
admin bootstrap passwords are read from the platform project.

The workflow expects these GitHub Actions variables, set from the
[`infra/gcp/apps/`](infra/gcp/apps/README.md) Terraform outputs:

- `APPS_GCP_WORKLOAD_IDENTITY_PROVIDER`
- `APPS_CI_SERVICE_ACCOUNT`
- `APPS_DEPLOY_SERVICE_ACCOUNT`

CI and deployment also require the repository Actions secret
`FIRNA_REPOSITORY_TOKEN`. It should contain a machine token restricted to
reading the private `futex-ai/firna` platform repository. Google Cloud IAM does
not grant access to private GitHub source dependencies.

Ephemeral `pr-N` platform previews are seeded by the platform repository from
a checkout of this repository at `main`, driven by the same `deploy.toml`
targeting. The full contract is
[`docs/protocol/app-deployment.md`](docs/protocol/app-deployment.md).

## Key Code

- [`apps/`](apps/README.md): manifests, component source, assets, and runtime
  tests.
- [`scripts/repository_audit.py`](scripts/repository_audit.py): manifest
  authoring-contract, compatibility, version, source-layout, and file-length
  checks.
- [`scripts/plan-app-deploys.py`](scripts/plan-app-deploys.py): catalog-aware
  production deployment planning.
- [`deploy.toml`](deploy.toml) and [`scripts/deploy_config.py`](scripts/deploy_config.py):
  environment targeting validated by the repository audit.
- [`infra/gcp/apps/`](infra/gcp/apps/README.md): Terraform for the dedicated
  app-secrets Google Cloud project and its workload identities.
- [`xtask/`](xtask/README.md): local and CI verification entrypoints.

The platform-side manifest, runtime, and admin submission contracts remain in
the [Firna platform app protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md).
The repository-specific [X app protocol](docs/protocol/x-app.md) defines its
OAuth, read, publishing, recovery, and cost-control contract. X OAuth client
credentials are deployment-supplied so production and the stable `br-main`
preview can use separate provider apps with the same immutable package. The
[GitHub app protocol](docs/protocol/github-app.md) defines its installation,
tool, signed-event, lifecycle, and redaction contract. The
[app deployment protocol](docs/protocol/app-deployment.md) defines the
provisioning and deployment automation contract.
