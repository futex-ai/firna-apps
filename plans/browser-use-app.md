# Browser Use App

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-11

## Outcome

Add an explicitly installed first-party Browser Use app that starts, inspects,
and cancels bounded Browser Use Cloud V4 agent runs. Firna supplies the
provider account, reserves at most $1.00 from the authorizing workspace before
dispatch, and settles the exact final Browser Use `totalCostUsd` through the
existing app-task billing ledger. Status and cancel calls remain free and may
settle the original task exactly once.

The complete product and provider contract is
[`docs/protocol/browser-use-app.md`](../docs/protocol/browser-use-app.md).
Implementation must stay inside that V1 boundary: one-off runs only, fixed
model and cost cap, no profiles, reusable sessions, files, recordings, live
browser links, Browser Use integrations, schedules, user credentials, or
workspace-owned provider keys.

Creating and funding Browser Use environments, adding provider and HMAC keys to
Google Secret Manager, merging the prerequisite platform change, approving the
initial live provider spend, merging the app release, and running a real paid
smoke task are external actions. Each requires the explicit human decision
listed in the relevant milestone. Local builds and tests never contact Browser
Use or incur provider cost.

## Planning Baseline

- [x] Review the repository app, deployment, secret, pricing, task-settlement,
  package-test, and post-push review conventions.
- [x] Review the platform revision pinned by `platform.toml` and current
  platform `main`; both can reserve and sweep long-running priced app tasks.
- [x] Identify the platform gap: only a directly priced or task-opening tool
  currently receives the typed result decoder, so a free get/cancel tool cannot
  report terminal usage.
- [x] Check Browser Use's V4 create, status, result, cancel, authentication, and
  current pricing contracts on 2026-08-11.
- [x] Add the complete Browser Use app protocol before creating this plan, keep
  it below 250 lines, and link it from this plan.

## Product and Technical Contract

- Package id `browser-use`, name `Browser Use`, version `1.0.0`, source kind
  `built_in`, install policy `explicit`, and single workspace installation.
- Expose exactly `browser_use_start_task`, `browser_use_get_task`, and
  `browser_use_cancel_task`. Classify start and cancel conservatively as
  external writes; a natural-language browser goal may mutate an external
  site, and cancellation changes provider state.
- Use Browser Use V4 at `api.browser-use.com`, model `gpt-5.6-luna`, a US
  managed proxy, recording disabled, and provider `maxCostUsd` fixed to the
  manifest's $1.00 task cap.
- Inject the app-owned `api_key` only into
  `X-Browser-Use-API-Key`. Use a distinct app-owned `task_handle_key` through
  the opaque HMAC host capability to bind every returned task handle to its
  Firna installation.
- Keep create asynchronous. Start returns a signed handle and a status; get
  uses the cheap status endpoint before fetching a terminal summary; cancel
  calls the provider's documented idempotent cancel endpoint once.
- Report the final cost for completed, failed, and cancelled provider runs.
  Parse the decimal string without floating point, round upward only below one
  micro-USD precision, and never expose cost or token counts in tool output.
- Reserve the task cap before dispatch, settle against the opening call's
  pinned app version, clamp and audit an upstream over-cap value, and leave
  unresolved or malformed usage in reconciliation rather than releasing or
  inventing a charge.
- Treat a create response lost after dispatch as `task_outcome_unknown`. Do not
  redispatch. Browser Use V4 has no documented idempotency or client-reference
  field, so opening-operation recovery remains fail-closed and operator-led.
- Bound task input, provider responses, returned results, component runtime,
  and every error field exactly as the protocol specifies. Never pass through
  raw provider JSON, errors, live URLs, internal ids outside signed handles, or
  credential material.
- Reuse the platform's generic catalog, install consent, Usage, Billing, and
  app-detail surfaces. No Browser Use-specific UI or mockup is required because
  those reviewed surfaces already render task caps, durations, reservations,
  and app spend from typed manifest and ledger data. If implementation exposes
  a missing user-facing state, add a separate tagged mockup milestone before a
  separate tagged UI milestone; do not fold UI work into a backend milestone.

## Milestone 1: Land Reports-Only Task Settlement in Firna

Implement this prerequisite in a separate workspace for the private Firna
platform repository. At the end of the milestone, the platform remains fully
usable and can safely accept a free observer tool that reports usage for an
already-open priced task. Do not point `firna-apps` at an unmerged revision.

- [ ] Update `docs/protocol/app-usage-pricing.md` and the app manifest contract
  to define `pricing.tools[].reports_tasks`, including its interaction with
  direct tool pricing, `opens_task`, task consent, charging flags, and public
  pricing projections.
- [ ] Add the typed optional field to the manifest interface with backward-
  compatible serde behavior; validate non-empty unique declared task ids and
  require every pricing tool entry to have a charge, an opened task, or a
  reported task.
- [ ] Extend the Wasm result decoder so only a tool's declared
  `reports_tasks` may emit terminal lifecycle events. Preserve the existing
  rule that only `opens_task` may emit `task_opened`.
- [ ] Route reports-only calls through the typed result envelope without taking
  a new hold, creating a per-call usage event, requiring new spendable credit,
  or presenting the tool as separately priced.
- [ ] Allow reports-only calls and the abandonment reporter to reconcile holds
  that were opened before new app charging became unavailable; keep all new
  task-opening calls fail-closed while charging is unavailable.
- [ ] Resolve every terminal event against workspace, app, installation, task,
  provider id, opening operation, and pinned app version before checkpoint and
  settlement. Reject unknown, cross-installation, cross-version, wrong-kind,
  overlong, or conflicting events without exposing another workspace's task.
- [ ] Add interface, manifest validation/serde, runtime decode, billing hold,
  settlement, replay, charging-disabled, multi-connection, public projection,
  redaction, and failure-injection tests. Use `unimock` at every impure unit-
  test boundary.
- [ ] Update affected crate READMEs and protocol links, keep Rust files below
  300 lines, and run the platform's complete format, lint, unit, integration,
  migration, and `cargo xtask check` gates with a 100% pass rate.
- [ ] In the platform workspace, run `git add -A`, commit with a Conventional
  Commit, push without renaming its branch, then run `cargo xtask review`.
  Report every finding for human decision without automatically fixing it.
- [ ] Merge only after the prerequisite's CI, review findings, and human
  approval are resolved; record the full merged platform commit for the next
  milestone.

## Milestone 2: Pin the Merged Platform Contract

Update this repository only after Milestone 1 is merged. At the end of the
milestone, all existing apps still build and test against one canonical
platform revision that contains `reports_tasks`.

- [ ] Update `platform.toml`, the deployment workflow revision, every
  standalone platform-runtime manifest, and all corresponding lockfiles to the
  same full merged commit.
- [ ] Bump every existing app version required by the repository's changed-app
  audit when its runtime test dependency changes; do not republish immutable
  package versions.
- [ ] Add or update repository-audit regressions so partial platform pins or a
  reports-only manifest against an incompatible revision fail locally.
- [ ] Update affected app/runtime READMEs with only useful compatibility
  context; do not claim the Browser Use package exists yet.
- [ ] Run formatting, every existing component and platform-runtime test, the
  repository audit, and `cargo xtask check` before beginning the new package.

## Milestone 3: Provision Isolated Provider Environments

Provision external state only after the app id, secret names, API version, and
cost cap are fixed. At the end of this milestone, production and stable preview
can supply the app's two secrets, but the app is not deployed and no provider
secret has entered Git, shell output, screenshots, `.context`, or chat.

- [ ] In Browser Use Cloud, create separate production and stable-preview
  projects or accounts with distinct API keys and independently inspectable
  usage. Do not share one key or credit pool across environments.
- [ ] Recheck V4 model availability, create/status/get/cancel shapes,
  `maxCostUsd` enforcement, current prices, account concurrency, retention, and
  account-level budget controls in the provider dashboard and official docs.
  Stop for a protocol update if any fixed assumption differs.
- [ ] Ask the human operator to approve the exact initial top-up, account spend
  limit, and any automatic recharge separately for production and preview.
  Keep automatic recharge off unless explicitly approved.
- [ ] Generate independent high-entropy `task_handle_key` values for production
  and preview. Establish a rotation runbook that never invalidates handles for
  open tasks without first cancelling/reconciling those tasks.
- [ ] Add enabled secret versions for
  `prod-app-browser-use-api-key`,
  `prod-app-browser-use-task-handle-key`,
  `preview-app-browser-use-api-key`, and
  `preview-app-browser-use-task-handle-key` in the `firna-apps` Google Cloud
  project using direct secret input that does not echo values.
- [ ] Verify only metadata, enabled version numbers, environment separation,
  service-account access, credit balance, and approved budget controls. Never
  copy a secret value into evidence or the implementation workspace.

## Milestone 4: Build the Browser Use Package

Implement the protocol under `apps/browser-use`. At the end of the milestone,
the standalone component builds for Wasm, all native component tests pass, and
the package contains no real provider data or credentials.

- [ ] Add the official, permission-compatible Browser Use mark as a square PNG,
  base64 sidecar, and manifest icon with an accessible colour pair. Verify the
  three representations are byte-identical and do not invent a replacement
  brand mark.
- [ ] Add `manifest.yaml` version `1.0.0` with explicit install, built-in
  source, least-privileged HTTP methods/host/header, HMAC capability, two
  app-owned secrets, the exact three bounded tools, activity labels, side-
  effect classifications, response/runtime limits, reports-only task entries,
  and the immutable $1.00/one-hour usage-reported task declaration.
- [ ] Create the standalone Rust component workspace and lockfile using the
  repository's established ABI. Keep `lib.rs` and module roots thin, add module
  and public API docs, use typed structs/enums throughout, and keep every Rust
  file below 300 lines.
- [ ] Put provider HTTP, HMAC signing, and other impure behavior behind traits;
  consume trait objects in services and use `unimock` for native unit tests.
  Define module-local `thiserror` enums and preserve stable typed causes.
- [ ] Add tests first for manifest-bound validation, exact V4 request shapes,
  create success, every provider status, terminal summaries, result truncation,
  decimal-to-micro-USD conversion, cap overflow, and zero cost.
- [ ] Implement installation-bound versioned handles with exact parsing,
  constant-time HMAC comparison, cross-installation rejection, malformed input
  rejection, and no provider call before ownership validation.
- [ ] Implement start with one non-retried create request and one
  `task_opened` event; distinguish pre-dispatch provider errors from
  crash-ambiguous outcome and never synthesize a provider task id.
- [ ] Implement get with one lightweight status request and a full summary only
  when terminal. Implement cancel with one provider cancel request. Emit final
  reported cost for completed, failed, and cancelled runs without exposing
  financial or raw provider fields.
- [ ] Implement `report-task-usage` for trusted provider-task lookups. Return
  running/terminal precisely; make opening-operation lookup fail closed until
  Browser Use documents a safe correlation mechanism, and never return absent
  from an unprovable lookup.
- [ ] Add typed redacted handling for validation, authentication, not-found,
  rate-limit, provider-credit, timeout, 4xx, 5xx, truncated, malformed, and
  unknown provider responses. Ensure terminal cost failures leave the original
  hold reconcilable.
- [ ] Add concise app and component READMEs with responsibilities, quick-start
  commands, public tool contract, billing/recovery behavior, security boundary,
  development commands, key code, related docs, and V1 non-goals.
- [ ] Run component format, clippy, native unit tests, locked Wasm build, and
  Wasm validation until every check passes with no warnings or failures.

## Milestone 5: Verify Packaging, Runtime, and Billing

Exercise the real packaged component through the pinned platform runtime. At
the end of the milestone, the new app is locally installable, its task lifecycle
and billing are proven end to end with fakes, and repository documentation
matches the package.

- [ ] Add a standalone `apps/browser-use/tests/platform-runtime` workspace,
  lockfile, README, fixtures, and target-directory isolation consistent with
  every existing package.
- [ ] Assert manifest validation, exact source/install/capability/secret/tool
  schemas, signed-handle limits, task cap/duration, `opens_task`,
  `reports_tasks`, usage reporter export, icon identity, and absence of
  undeclared auth, ingress, and capabilities.
- [ ] Prove host injection puts the opaque app-owned API key only in
  `X-Browser-Use-API-Key`, HMAC never releases key material, and no component
  request or result contains either secret.
- [ ] Exercise start, running get, terminal get, repeated terminal get, cancel,
  failed/cancelled terminal runs, and abandonment reporting through the real
  Wasm runtime with captured fake provider responses.
- [ ] Prove the opening call reserves exactly $1.00 before provider dispatch;
  reports-only calls reserve and charge zero; first terminal usage settles the
  exact rounded cost; replay is a no-op; over-cap cost clamps/audits; and all
  failure paths preserve or release the correct hold.
- [ ] Add regressions for insufficient credit, charging disabled before open,
  charging disabled after open, cross-workspace/install task ids, invalid
  handles, missing task registry rows, ambiguous create, reporter retries, and
  reconciliation-required transition without cross-tenant output leakage.
- [ ] Add `apps/browser-use/deploy.toml` targeting production and stable preview
  only. Confirm ephemeral previews neither deploy nor invoke this paid app.
- [ ] Update root and `apps/` catalog documentation plus the protocol with the
  final implemented version and limits. Keep current catalog wording accurate
  until the package is actually present.
- [ ] Run `firna apps validate apps/browser-use` and
  `firna apps package apps/browser-use`; inspect the deterministic source bundle
  and prove it excludes secrets, local targets, and stale vendor/config trees.
- [ ] Smoke the packaged Wasm against fake create, running, terminal, repeat,
  and cancel responses. No local or CI smoke may contact Browser Use.

## Milestone 6: Run the Complete Repository Gate

Require the entire repository to remain green before committing the feature.

- [ ] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
  rerun the check before proceeding.
- [ ] Run `cargo xtask rust-file-length-lint --all` if the command exists;
  otherwise run the supported repository-audit command that includes the Rust
  file-length and structural checks, and record the substitution.
- [ ] Run every targeted component, platform-runtime, package, deployment,
  secret, documentation, and audit test discovered during implementation, then
  run `cargo xtask check` and require a 100% pass rate.
- [ ] Resolve every compile, lint, test, package, documentation, or fake smoke
  failure. If an external provider blocks only live verification, record the
  exact blocker and all local checks that passed.
- [ ] Fetch `origin/main`, inspect mainline changes from the merge base, and
  resolve overlaps path by path without removing or overwriting unrelated work.
- [ ] Inspect `git diff --check`, the complete diff, `git diff --name-status
  origin/main`, and the deletion-only diff; stop unless every deletion is
  explicitly part of the reviewed plan.

## Milestone 7: Commit and Push the Checked Work

- [ ] Run `git add -A` so every manifest, component, asset, sidecar, test,
  lockfile, protocol, README, deployment file, and completed plan update is
  tracked.
- [ ] Commit with a Conventional Commit title and a body describing the V4
  boundary, asynchronous task billing, ownership handles, cost cap, recovery,
  tests, and non-goals. A suitable title is
  `feat(browser-use): add task app`.
- [ ] Push the current branch without renaming it.
- [ ] Inspect `git diff --name-status origin/main..HEAD` and the deletion diff
  after the commit; stop if an unauthorized mainline removal is present.

## Milestone 8: Run Post-Push Review

- [ ] After the push, run `cargo xtask review` so the AI reviewer evaluates the
  committed diff against `origin/main`.
- [ ] Independently investigate every finding, but do not automatically change
  reviewed code. Report each finding as a numbered item with severity,
  codebase and feature context, impact of doing nothing, lettered solution
  options, and a clearly recommended option.
- [ ] For every recommendation, assess whether a direct fix is sufficient or a
  broader test, lint, rule, abstraction, or architectural change would better
  prevent the same class of defect. Wait for the user to choose fixes.

## Milestone 9: Release and Live Paid Smoke Test

Release through the normal `main` workflows only. At the end of this milestone,
Browser Use is present in both intended catalogs and one bounded real task has
proved provider execution and workspace-wallet settlement.

- [ ] Merge only after required CI, platform compatibility, secret gate,
  review decisions, and human approval; do not deploy from the workspace branch.
- [ ] Verify version `1.0.0` in production and stable `br-main` after their
  normal deployment workflows complete, with the expected environment-specific
  secret versions and app charging enabled.
- [ ] Ask the human operator to approve one public, non-destructive Browser Use
  goal and its $1.00 maximum reservation. Do not use accounts, credentials,
  forms with side effects, payments, private data, or local/internal URLs.
- [ ] Start the goal in stable preview, observe at least one running status,
  fetch the terminal result, and repeat the terminal read. Confirm the second
  read creates no second charge.
- [ ] With separate approval, start a cheap second public goal and cancel it;
  confirm Browser Use stops further work and Firna settles only the provider's
  final consumed cost.
- [ ] Compare the provider run cost, micro-USD rounding, Firna task reservation,
  settled workspace charge, released hold, Usage app row, Billing transaction,
  provider credit balance, and account spend limit. Retain only redacted ids,
  states, and aggregate amounts.
- [ ] Verify no open or reconciliation-required task remains after smoke
  testing. Exercise the documented operator runbook if one does.
- [ ] Move this plan from Active to Completed in `plans/README.md` only after
  every milestone, live validation task, and approved review-fix milestone is
  complete.

## Completion Criteria

- Browser Use is an explicit first-party app with exactly three bounded tools
  and no V1 capability outside the completed protocol.
- Every provider run has one installation-bound handle, one finite wallet hold,
  one final reported cost, and at most one settlement, including failure and
  cancellation outcomes.
- Free observation and cancellation calls can settle an existing task without
  taking new credit, and cross-workspace/provider ids cannot leak or settle.
- Provider ambiguity, malformed cost, and reporter failure remain fail-closed
  and operable; no retry duplicates an external run or silently releases a
  financially unresolved hold.
- Component, Wasm, runtime, package, deployment, smoke, and complete repository
  checks pass; all files are committed and pushed before post-push review.
- Every review finding is presented for user decision, and the normally
  deployed app is verified with redacted live billing evidence.
