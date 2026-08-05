# App Deployment and Provisioning

- Status: target contract, implemented by
  [`plans/inbuilt-app-deploy-automation.md`](../../plans/inbuilt-app-deploy-automation.md).
  Until that plan completes, the repository `README.md` describes live behavior.

This document defines how first-party app packages reach Firna's long-lived
environments with no per-app platform-repository changes. After a package
merges to `main` here, it must appear in every targeted environment either
immediately (environments this repository deploys) or on the next platform
deploy (ephemeral previews seeded by `futex-ai/firna`).

## Ownership Boundary

`firna-apps` owns:

- app packages, manifests, and per-app deployment targeting;
- the dedicated app-secrets Google Cloud project, its Terraform, and its
  workload identities;
- the merge gate that blocks a PR until every declared secret has a value;
- deployment to the long-lived environments `production` and `br-main`.

`futex-ai/firna` owns:

- the platform contract (manifest schema, submit/build/install path);
- seeding ephemeral `pr-N` previews from a `firna-apps@main` checkout using
  the pull request's own CLI build;
- the two admin bootstrap-password secrets, with per-secret read grants to
  this repository's deploy identity.

The platform repository must never enumerate app ids, app secret names, or
app environment targeting. Its preview seeder derives all three from the
`firna-apps` checkout it already makes.

## Environments Model

Environments are a deployment-topology concept owned by this repository's
tooling. They are not part of the platform apps system: manifests, catalog
rows, and installed apps remain environment-agnostic, and each environment is
a fully isolated platform instance (own catalog, database, installations,
artifacts, credentials).

Each environment instance has a class. The class selects which secret values
an instance receives and which apps may deploy to it.

| Instance   | Class        | Deployed by  | API URL                                  |
| ---------- | ------------ | ------------ | ---------------------------------------- |
| production | `production` | `firna-apps` | `https://api.firna.ai`                   |
| br-main    | `preview`    | `firna-apps` | `https://br-main.api.preview.firna.ai`   |
| pr-N       | `preview`    | `firna`      | `https://pr-N.api.preview.firna.ai`      |

## Deployment Configuration

The repository root `deploy.toml` declares the app-secrets project and the
long-lived instances this repository deploys:

```toml
[gcp]
project_id = "<app-secrets project id>"
platform_project_id = "firna-498513"

[environments.production]
class = "production"
api_url = "https://api.firna.ai"
secret_prefix = "prod-app"
admin_email = "admin"
bootstrap_password_secret = "firna-prod-runtime-firna-bootstrap-password"

[environments.br-main]
class = "preview"
api_url = "https://br-main.api.preview.firna.ai"
secret_prefix = "preview-app"
admin_email = "preview-admin"
bootstrap_password_secret = "firna-preview-test-runtime-firna-bootstrap-password"
```

`gcp.project_id` is the dedicated app-secrets project. Bootstrap-password
secrets live in `gcp.platform_project_id` and are the only values read
outside the app-secrets project.

An app opts out of a class with an optional `apps/<app_id>/deploy.toml`:

```toml
classes = ["production"]
```

A missing per-app file means the app targets every class. Targeting is
operational metadata for Futex's release automation; it is intentionally not
part of the platform manifest contract, is never packaged into source
bundles, and never reaches the platform. `x` is the only current exception:
its X developer app registers only the production OAuth callback and
credential, so it declares `classes = ["production"]`.

`cargo xtask check` validates both files: the root file must declare exactly
the instances above with well-formed URLs and prefixes, and per-app files
must contain a non-empty `classes` array whose entries are known classes.

## App-Secrets Project

A dedicated Google Cloud project holds only app-provider secret containers.
Its Terraform lives in this repository under `infra/gcp/apps/` and manages:

- Workload Identity Federation for this repository only: pool `github`,
  provider `github-firna-apps` restricted to `futex-ai/firna-apps`;
- service account `apps-ci`, assumable from any ref of this repository, with
  `roles/secretmanager.viewer` plus a custom `appSecretCreator` role holding
  only `secretmanager.secrets.create`;
- service account `apps-deploy`, assumable from `refs/heads/main` only, with
  `roles/secretmanager.secretAccessor` on the project;
- a cross-project binding granting `github-firna-preview@firna-498513`
  `roles/secretmanager.secretAccessor` under an IAM condition restricting it
  to `preview-app-` secret names, for `pr-N` seeding.

Secret containers are named `<secret_prefix>-<app_id>-<secret-name-kebab>`,
for example `prod-app-x-client-secret` and `preview-app-exa-api-key`.
Manifest secret names use lower snake case and are kebab-cased in container
ids.

`apps-ci` can create empty containers and read metadata but can never read
secret values; pull-request CI therefore holds no credential that exposes a
provider key. Secret creation cannot be prefix-scoped by IAM conditions, so
container creation is safe to grant only because the project contains
nothing except app secrets. Humans add values with
`gcloud secrets versions add`; automation never writes secret values.

## Merge Gate

A `ci.yml` job authenticates as `apps-ci` and runs
`scripts/check_app_secrets.py`, which:

1. reads every `apps/*/manifest.yaml` `secrets` entry (names only; full
   manifest validation remains `cargo xtask check`'s job) and each app's
   targeted classes;
2. for each targeted class and declared secret, creates the container if it
   does not exist, labeled `app=<app_id>`;
3. verifies the container has an enabled latest version;
4. fails listing every missing value with the exact remediation command, for
   example
   `printf '%s' "$VALUE" | gcloud secrets versions add preview-app-slack-signing-secret --project=<id> --data-file=-`.

A pull request that adds or changes an app cannot merge until every value it
needs in every targeted environment class exists. After merge, deployment
can never stall on provisioning. The gate runs on pull requests from this
repository and on `main`; it does not run for forks, which cannot mint the
repository's identity token.

An empty-string secret version passes the metadata-only gate but fails
closed at deploy time, which reads values.

## Deployment Workflow

`deploy-apps.yml` deploys every environment instance in root `deploy.toml`.
Triggers:

- successful `CI` `workflow_run` on `main` (existing);
- `workflow_dispatch` with optional app id and optional single instance
  (existing force path, extended);
- daily `schedule`, so a reset or externally rebuilt environment converges
  without platform coordination;
- `repository_dispatch` type `firna-platform-deployed`, an optional poke the
  platform deploy sends after rebuilding an environment.

Per instance, the workflow authenticates as `apps-deploy`, reads the
instance's bootstrap password from the platform project, logs in as the
instance's admin, plans against that instance's catalog with
`scripts/plan-app-deploys.py` filtered to apps targeting the instance's
class, and submits missing or newer packages with
`firna admin apps submit`, passing each declared secret from
`<secret_prefix>-<app_id>-<secret-name-kebab>` by environment variable.

Fail-closed rules:

- the workflow refuses any API URL not exactly equal to a root `deploy.toml`
  instance URL, and refuses `preview` secret prefixes against `production`
  instances and vice versa;
- a missing or empty secret value fails that instance's deployment;
- instances deploy independently, so one failing instance does not block the
  other.

Version skew: this repository's CLI is pinned by `platform.toml` while
`production` and `br-main` servers track `firna@main`. Both instances sit in
the same skew envelope, so compatibility with current platform `main` is a
single invariant. If a platform change breaks the pinned CLI, both
deployments fail loudly and the fix is one `platform.toml` bump. The
scheduled run doubles as a drift canary: it exercises catalog and submit
RPCs against both instances daily.

## Platform Integration (`futex-ai/firna`)

- `pr-N` preview seeding checks out `firna-apps@main`, derives the app list
  from manifests plus per-app `deploy.toml` (`preview` class only), builds
  and validates with the pull request's own CLI, and reads values from
  `preview-app-*` in the app-secrets project. No app-id allowlist
  environment variables remain.
- The platform grants this repository's `apps-deploy` identity per-secret
  read on exactly two secrets: the production and preview-test bootstrap
  passwords.
- After a platform deploy rebuilds `production` or `br-main`, the platform
  may send `repository_dispatch` `firna-platform-deployed` to this
  repository; the daily schedule is the fallback when it does not.

## Legacy and Migration

The platform project's `firna-prod-app-*` and app-provider
`firna-preview-test-runtime-*` containers, the platform Terraform
`app_provider_keys` list, the `FIRNA_PREVIEW_APP_IDS` allowlists, the
platform-owned deploy identity for this repository, and the External Secrets
Operator's unused `-app-` prefix grant are legacy. They remain untouched
until the plan's cutover milestones verify the new path, then are removed in
the platform repository. Secret values are copied, never moved, so rollback
at any milestone is reverting workflow configuration.
