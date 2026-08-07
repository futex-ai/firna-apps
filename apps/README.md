# Apps

`apps/` contains trusted, Firna-owned app packages. The packages live in this
repository so the platform and community app catalogs do not own first-party
source.

## Package Layout

Each app lives at `apps/<app_id>`. The directory name must match the manifest
`id`; CI uses the directory name for submit and production secret lookup.

- exactly one root manifest: preferred `manifest.yaml` with public `env` values
  and required `secrets` names, or compatibility `manifest.json`. Local
  validation rejects an app containing both formats.
- `component/`: source and `Cargo.lock` that the isolated Firna app-builder
  service compiles to Wasm.
- `assets/`: app-owned images or static files.

The source bundle uploaded by `firna apps package` must be a deterministic
`.tar.gz` archive rooted at the app directory. Production CI uploads source and
the parsed manifest as JSON; it does not upload a developer-built production
Wasm component. Packaging honors `.gitignore` rules and vendors the
component's locked Cargo dependencies into `component/vendor` so the builder can
run offline. Any source-owned `component/vendor` or `component/.cargo` tree is
excluded from the source pass; those paths contain only the packager's fresh
offline build inputs in the completed archive.

## Local Commands

```bash
firna apps new apps/demo --app-id demo --name Demo --non-interactive
firna apps validate apps/slack
firna apps validate apps/exa
firna apps validate apps/github
firna apps validate apps/http
firna apps validate apps/dataforseo
firna apps validate apps/x
firna apps package apps/slack
cargo test --manifest-path apps/slack/tests/platform-runtime/Cargo.toml --locked
cargo test --manifest-path apps/exa/tests/platform-runtime/Cargo.toml --locked
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
cargo test --manifest-path apps/http/tests/platform-runtime/Cargo.toml --locked
cargo test --manifest-path apps/dataforseo/tests/platform-runtime/Cargo.toml --locked
cargo test --manifest-path apps/x/tests/platform-runtime/Cargo.toml --locked
```

Install the compatible CLI revision documented in the
[repository README](../README.md) before running `firna` commands.

`firna apps new` makes the generated component a standalone Cargo workspace and
creates its lockfile immediately. The command therefore works under `apps/`
without adding the component to this repository's root workspace.

When using `--secret-file`, keep the private input file outside the submitted
app directory so it cannot become part of the source bundle. `firna` rejects a
selected secret file whose resolved path is inside the app source tree before
opening or parsing its contents.

## Manifest Conventions

Prefer YAML. `env` values are public config values applied during admin submit.
`secrets` contains names only; secret values are supplied to
`firna admin apps submit` with `--secret-env`, `--secret-stdin`, or a private
`--secret-file`.

Every app icon declares a `color_pair` with `primary` and `secondary`
six-digit sRGB hex colours. Product surfaces use the package icon and these
colours together—for example, the animated ring around an app icon while its
tool runs. Choose a pair that remains distinct around the icon at small sizes.

Every tool declares a public-safe, task-specific `activity_label` for the
compact chat status shown while that exact tool runs. Labels use two or more
single-space-separated printable ASCII words, contain no more than 80 bytes,
and start with a capitalized ASCII action word. The platform rejects the
removed `activity_verb` key; there is no compatibility alias or fallback.

App tools are available to every live agent member with installation access and
satisfied app authentication. New packages cannot declare the retired
`required_agent_permissions` block; the platform accepts it only while reading
immutable manifests stored under the old contract.

Webhook-capable packages declare provider events in the `events` list nested
under their owning `ingress` entry. Each event owns a stable app-local id,
provider type, model-safe description, and positive contract version; the
containing ingress supplies its ingress id. Each webhook ingress lists the
exact lowercase request headers its verifier receives in `allowed_headers`.
The platform rejects the retired top-level `events` shape in new submissions;
packages without provider events omit `events` rather than declaring an empty
catalog. Packages do not declare handlers or subscriptions: agents explicitly
subscribe themselves after installation.

Production secret IDs use:

```text
firna-prod-app-<app_id>-<secret-name-kebab>
```

For example, `apps/slack` secret `client_secret` maps to
`firna-prod-app-slack-client-secret`.

Environment-specific public identifiers may also be required app-owned values
when they must vary without changing the package. X declares both `client_id`
and `client_secret` this way so production and stable preview deployments use
separate OAuth apps with the same manifest.

## Repo-Owned Apps

- `apps/slack`: explicit-install Slack workspace integration with OAuth,
  webhook, Slack tool support, and cyan/magenta icon accents.
- `apps/exa`: workspace-default Exa web-search app exposing
  `exa_web_search`; a workspace may supply its own Exa API key, while the
  app-owned `api_key` secret remains the zero-configuration fallback. Both
  values stay behind host-mediated credential injection.
- `apps/github`: production-only, explicit-install built-in GitHub App package
  for short-lived repository credentials, five bounded read tools, and six
  signed repository event definitions.
- `apps/http`: workspace-default built-in HTTP app exposing `http_request`.
  It uses the first-party broad HTTP host capability and does not receive or
  inject app/provider credentials.
- `apps/dataforseo`: explicit-install built-in research app exposing 16 bounded
  synchronous search, keyword, backlink, page, business, content, domain, and
  AI visibility tools. The installer supplies the workspace's DataForSEO API
  login/password pair through Settings; the package declares no app-owned or
  deployment secret.
- `apps/x`: explicit-install X integration with workspace-owned OAuth, bounded
  Post lookup and recent search, single-Post publishing, workspace-wallet usage
  charging, and operator spending controls defined by the
  [X app protocol](../docs/protocol/x-app.md).
