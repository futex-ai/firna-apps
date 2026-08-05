# X API Access

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-05

## Outcome

Add an explicitly installed first-party `x` app that lets a Firna workspace
authorize one X account, read bounded sets of posts, search recent posts, and
publish a text post or reply. The integration uses X OAuth 2.0 Authorization
Code with PKCE, keeps provider credentials behind Firna's opaque credential
boundary, and operates against X's credit-based pay-per-use API with explicit
cost and write-safety controls.

Creating the X Developer Console app, purchasing credits, and publishing a
live smoke-test post are external actions. The app creation is in scope, but
the exact credit purchase, billing-cycle spending limit, and public smoke post
each require a human confirmation at the milestone that performs the action.
Local builds and runtime tests never deploy the app. Live packages are released
through the normal production workflow and the platform's stable `br-main`
preview workflow from `main`; smoke validation happens only after the relevant
release.

## Current Constraints

- The X API currently uses prepaid credits and per-resource/per-action charges.
  Rates must be rechecked in the Developer Console immediately before purchase;
  the console is authoritative when it differs from documentation.
- As rechecked on 2026-08-02, X documents Post reads at $0.005 per returned resource,
  User reads at $0.010 per returned resource, text Post creation at $0.015 per
  request, and Post creation with a URL at $0.200 per request. Owned Reads are
  $0.001 per resource only when the authenticated X user is also the owner of
  the developer app; that discount must not be promised for other connected
  accounts.
- OAuth scopes are `tweet.read`, `tweet.write`, `users.read`, and
  `offline.access`. The offline scope returns a refresh token; X's user access
  token flow requires refresh-token rotation for a durable connection.
- The canonical Firna platform revision is `90d4c13f`, which contains the
  merged OAuth refresh lifecycle, usage-based app charging, and task-specific
  activity-label contracts required by this package, plus the canonical
  `required_agent_permissions.top_level` manifest contract.
- X does not document an idempotency key for `POST /2/tweets`. Firna's existing
  durable tool ledger replays completed results and fails crash-ambiguous
  installed-app calls closed without redispatch. X must reuse and verify that
  policy rather than add a second operation journal.
- Firna prepays X through the developer account and charges successful app
  calls to the authorizing workspace's Firna credit wallet at declared X list
  rates. X's daily resource deduplication is not observable per call, so Firna
  charges each returned resource at the declared app rate. Failed calls remain
  uncharged and their provider cost is borne by Firna.
- Platform PR [futex-ai/firna#1012](https://github.com/futex-ai/firna/pull/1012)
  merged as `ad4072fc` on 2026-08-04 after ten review invocations, 23 valid
  findings fixed with regressions, and two invalid plan-bookkeeping findings.
  The repository was initially pinned to platform `main` at `dbcc678a`, which
  also contains native usage-based app billing and activity labels, and was
  later compatibility-repinned to `90d4c13f`.
- This Conductor session has no controllable signed-in X console connector, so
  the human operator must confirm the production app type, callback, and OAuth
  settings. On 2026-08-04, the public client id was recorded in the manifest
  and the copied client secret was transferred directly from the clipboard
  into enabled Google Secret Manager version 1 of
  `firna-prod-app-x-client-secret`; the production apps deployment service
  account has secret-accessor permission, and no secret value was recorded.
  The user approved and funded a $10 one-time balance reserved for initial
  production validation, with a $10 billing-cycle cap and auto-recharge
  disabled.
- On 2026-08-05, the operator clarified that the latest OAuth2 client id and
  secret pair belongs to production. It was transferred into enabled version 2
  of `firna-prod-app-x-client-id` and `firna-prod-app-x-client-secret`, retaining
  version 1 for rollback. The same pair had initially been assigned to preview
  in error, so version 1 of `firna-preview-test-runtime-x-client-id` and
  `firna-preview-test-runtime-x-client-secret` was disabled. The earlier
  staging consumer-key pair is an OAuth 1.0 credential type and is not used by
  this app's OAuth2 flow. Dedicated staging OAuth2 values are still required;
  no credential value was recorded.

Official references:

- [X API pricing](https://docs.x.com/x-api/getting-started/pricing)
- [X OAuth 2.0 Authorization Code with PKCE](https://docs.x.com/fundamentals/authentication/oauth-2-0/authorization-code)
- [X OAuth user access and refresh tokens](https://docs.x.com/fundamentals/authentication/oauth-2-0/user-access-token)
- [X developer apps](https://docs.x.com/fundamentals/developer-apps)
- [X create Post endpoint](https://docs.x.com/x-api/posts/create-post)
- [X recent Post search](https://docs.x.com/x-api/posts/search/introduction)

## V1 Product Contract

- One workspace administrator connects one X account for the workspace.
  Authorized workspace agents use that shared installation. Per-member X
  accounts are out of scope for V1.
- The app is explicit-install and requests only the four OAuth scopes listed
  above. It does not request Direct Message, follow, like, list, bookmark,
  media, or moderation permissions.
- The app exposes exactly these initial tools:
  - `x_get_posts`: fetch 1-10 posts by ID, with optional author expansion.
  - `x_search_recent_posts`: search the recent index with an explicit page size
    of 10-25 and an optional provider pagination token.
  - `x_create_post`: publish one text post or one reply; no quote posts, media,
    polls, editing, deletion, or threads in V1.
- Read results are compact typed objects. Default requests omit author profile
  expansions and engagement metrics because those can add billed resources.
  Pagination is always initiated by a new explicit tool call; the component
  never drains pages automatically.
- `x_create_post` accepts bounded text and an optional numeric reply target.
  Link-bearing text requires an explicit `allow_link` input because X currently
  charges its higher link-Post rate. The console spending limit remains the
  authoritative protection because URL detection is not a billing oracle.
- Provider errors map to stable results: invalid input, authorization required,
  insufficient scope, not found, rate limited, credit/quota exhausted,
  provider unavailable, and write outcome unknown. Raw provider bodies,
  credentials, and billing identifiers never reach agent-visible output.
- No background polling, streams, webhooks, automatic retries, or scheduled
  reads are introduced. Every billed provider request corresponds to an
  explicit tool invocation or the minimum OAuth token lifecycle work required
  to keep an authorized connection usable.

## Milestone 1: Complete the X App Protocol

Document the exact contract before implementation. The repository remains
fully functional at the end of this documentation-only milestone.

- [x] Add `docs/protocol/x-app.md`, kept below 250 lines, covering manifest
  auth, tool schemas, provider endpoints, bounded outputs, pagination, error
  mapping, credential refresh, operation idempotency, and cost controls.
- [x] Record the V1 choice of one workspace-owned X account and the three-tool
  surface; do not silently widen the app to per-user grants or extra write
  actions during implementation.
- [x] Specify the stable behavior for an ambiguous create-Post result: never
  retry automatically, return `write_outcome_unknown`, and direct the caller to
  inspect X before issuing a new operation.
- [x] Specify how author expansion changes billed resources and ensure it is
  opt-in in both the manifest schema and component request.
- [x] Confirm public catalog availability with workspace app charging. The app
  remains explicit-install and moves to first-party `built_in` distribution in
  Milestone 6 because V1 rejects pricing on community manifests.
- [x] Link the protocol from the root and app documentation where relevant.
- [x] Validate Markdown links and review the documentation diff.

## Milestone 2: Land Firna Platform Prerequisites

Implement these changes in a separate workspace for the private Firna platform
repository. Do not point this repository at an unmerged or unreviewed platform
revision. The platform must remain green and independently usable after this
milestone.

- [x] Extend the app protocol's `standard_oauth2` contract with typed access
  token expiry and refresh behavior, including access-token and refresh-token
  credential kinds, `expires_in` mapping, and reuse of the reviewed token URL
  and client-auth method.
- [x] Persist token expiry metadata without exposing token values; refresh
  proactively near expiry and atomically rotate both tokens when X returns a
  new refresh token.
- [x] Serialize concurrent refreshes per installation, retry a provider request
  at most once after a successful refresh, and turn terminal `invalid_grant`
  into `auth_required` without leaking the provider response.
- [x] Verify the existing installed-app write recovery contract: completed
  durable results are replayed, while a crash-ambiguous pending operation fails
  closed without component or provider redispatch. Preserve the operation id in
  the interruption result so callers can reconcile it.
- [x] Put refresh, credential vault, clock, and provider HTTP behavior behind
  traits and use `unimock` at unit-test boundaries.
- [x] Add protocol, store, migration, service, runtime, concurrency, redaction,
  and failure-injection tests, including refresh-token rotation plus existing
  fail-closed coverage immediately before and after provider dispatch.
- [x] Update all affected Firna protocol docs and crate READMEs, then run the
  platform's complete checks, commit, push, and review workflow.
- [x] Inject OAuth refresh into task-reconciliation runtimes and fail closed
  when lifecycle-managed credentials cannot refresh.
- [x] Resolve refresh-time secrets with the workspace-internal storage app id.
- [x] Reject custom OAuth parameters that duplicate host-owned grant fields.
- [x] Make each lifecycle owner unambiguous across OAuth flows and credential
  kinds.
- [x] Reject empty initial lifecycle access and refresh token values before an
  installation can activate.
- [x] Omit refresh-claim coordination fields from store DTO serialization.
- [x] Bind lifecycle auth requirements explicitly to their standard OAuth flow.
- [x] Resolve later review findings covering claim fencing, terminal
  invalidation, reserved authorization parameters, provider error mapping,
  credential-pair validation, concurrent metadata replacement, migration
  compatibility, lifecycle reconciliation, and versioned single-credential
  KMS publication.
- [x] After the platform change lands, update `platform.toml`, the workflow
  pin, every standalone runtime-test manifest, and every lockfile together.
- [x] Preserve the latest mainline activity-label changes and advance each
  affected existing app version for its canonical platform repin.
- [x] Add or update repository-audit tests so a partial platform pin update is
  rejected.
- [x] Audit every declared `fna-*` runtime dependency, not only the required
  interface and Wasm pair, so host/store dependencies cannot retain stale pins.

## Milestone 3: Create and Secure the X Developer App

Provision the external app only after the callback and refresh contract are
fixed. At the end of this milestone the X app exists and its credential is in
the production secret store, but no secret appears in Git, shell history,
screenshots, logs, `.context`, or chat.

- [x] In `console.x.com`, create one production confidential Web App named
  `Firna`, using `https://firna.ai` as the website and plain-language copy that
  says Firna reads and publishes X posts only after workspace authorization.
- [x] Register the exact production callback
  `https://firna.ai/oauth/x/callback` with no trailing-slash variation.
- [x] Enable OAuth 2.0 Authorization Code with PKCE. Request scopes from the
  Firna manifest at authorization time rather than enabling unrelated X
  permissions.
- [x] Capture the public client ID for the manifest.
- [x] Transfer the one-time client secret directly into Google Secret Manager
      as `firna-prod-app-x-client-secret` without exposing the value, and grant
      the production apps deployment service account secret-accessor permission.
- [x] Before purchasing credits, ask the user to approve the exact initial
  credit amount and billing-cycle spending limit. Keep auto-recharge disabled
  unless the user explicitly approves an amount and trigger threshold.
- [x] Fund the production X developer account with the approved $10 one-time
  credit for initial live validation, set the billing-cycle spending limit to
  $10, and leave auto-recharge disabled.
- [x] Save a redacted record of the app id, callback, app type, approved budget,
  and secret version—not the secret value—in the implementation handoff.
- [x] Keep the initial `1.0.1` rollout production-only, with the production
  callback as its sole callback and no preview credential reuse.

Operator-confirmed handoff on 2026-08-04: production confidential Web App
`Firna`; website `https://firna.ai`; sole callback
`https://firna.ai/oauth/x/callback`; OAuth 2.0 Authorization Code with required
S256 PKCE and manifest-owned scopes; public client id recorded in the manifest;
Google Secret Manager secret `firna-prod-app-x-client-secret` version 1; $10
one-time balance and $10 billing-cycle cap; auto-recharge disabled. No secret
value or provider billing identifier is recorded.

This milestone records the initial production-only rollout. The separately
approved stable-preview app and isolated credentials were added on 2026-08-05;
they do not alter or reuse the production app configuration above.

## Milestone 4: Build the X App Package

Add a complete standalone package that can build and run through the pinned
Firna Wasm host. All provider data remains live or explicitly unavailable; no
reachable product path contains sample posts or metrics.

- [x] Create the initial `apps/x/manifest.yaml` with id/name `x`/`X`, version
  `1.0.0`, explicit installation, `api.x.com` as the only HTTP host, the
  app-owned `client_secret` declaration, and workspace-owned OAuth using
  `client_secret_basic` plus required S256 PKCE. Milestone 6 promotes its
  initial community source to priced first-party `built_in` distribution.
- [x] For the initial production-only package, put the real public client id in
  manifest environment key `client_id`; do not use a placeholder or secret
  value. The `1.0.2` stable-preview follow-up moves this environment-specific
  identifier to deployment-supplied app-owned storage.
- [x] Map `access_token`, `refresh_token`, granted scopes, and expiry through
  the platform's reviewed refresh contract. Store and inject tokens only by
  opaque credential reference.
- [x] Add an official X brand asset as SVG source, a PNG catalog icon under the
  manifest size limit, matching base64 source, and a legible color pair. Do not
  generate or redraw the X trademark.
- [x] Create the standalone Rust component and lockfile. Keep module roots thin,
  production and test files under 300 lines, public APIs documented, imports
  ordered, and all impure HTTP/runtime behavior behind trait objects.
- [x] Model requests, successful responses, pagination metadata, and provider
  errors with typed structs/enums. Do not retain or pass through arbitrary
  provider JSON beyond the HTTP boundary.
- [x] Implement the three V1 tools with strict input validation, bounded
  provider response reads, explicit pagination, compact outputs, stable error
  mapping, rate-limit metadata, and no implicit retries.
- [x] Make `x_create_post` issue at most one provider request per component
  invocation, never retry it in component code, and rely on the durable runtime
  ledger for completed-result replay and fail-closed crash recovery.
- [x] Add source-adjacent unit tests under `_tests_` using `unimock` for HTTP
  boundaries. Cover malformed calls, URL acknowledgement, provider errors,
  truncation, pagination, auth loss, and the no-retry write contract.
- [x] Add package, component, and platform-runtime READMEs with the required
  responsibilities, quick-start commands, key code, related docs, tool table,
  OAuth setup, cost behavior, secret name, and non-goals.
- [x] Update the root and `apps/README.md` catalog summaries and local command
  examples without turning them into exhaustive feature lists.

## Milestone 5: Add Runtime and Repository Verification

Exercise the real built Wasm component through the pinned platform host. The
new package and all existing packages must be green together.

The tracked runtime crate passes all 18 tests against the canonical merged
platform revision. On 2026-08-04, the pinned platform's completed-result store,
reclaimed-history, and fail-closed installed-app recovery regressions also
passed individually, composing with X's one-request component regression to
prove that recovery cannot issue a second create-Post request.

- [x] Add the `apps/x/tests/platform-runtime` source suite and
  publishable-quality README, and prove it locally against the reviewed
  platform branch.
- [x] Align the X manifest and local runtime OAuth lifecycle smoke with the
      final reviewed platform manifest, constructor, storage-identity, and
      credential-vault contracts, then rerun the complete local runtime harness.
- [x] After the platform change lands, add the standalone runtime
  `Cargo.toml` and lockfile using the canonical merged revision.
- [x] Validate the manifest id, version, icon, host allowlist, explicit install,
  OAuth owner/scopes/client method/PKCE/refresh mapping, tool names, schemas,
  side effects, and response limits.
- [x] Smoke every tool through the real Wasm component and a `WasmHostMock`;
  assert exact method, URL, query/body shape, opaque credential scope, output,
  and the absence of null or undeclared fields.
- [x] Test missing credentials, expired-token refresh, rotated refresh tokens,
  missing scopes, 401/403/404/429/credit exhaustion/5xx responses, malformed or
  truncated JSON, and redaction of provider details.
- [x] Prove at the platform runtime boundary that a repeated completed create
  operation returns its durable result without a second X request and an
  ambiguous pending operation never sends a second Post.
- [x] Treat malformed, missing, truncated, statusless, and 5xx create results
  as `write_outcome_unknown`, with component and real Wasm-runtime regressions,
  because X may already have accepted the Post.
- [x] Replace `xtask`'s hard-coded standalone manifest lists with filesystem
  discovery and test future component/runtime manifests plus the X component.
- [x] Make platform-pin auditing report a missing runtime lockfile cleanly
  instead of crashing, with a regression test for the partial-package state.
- [x] Once the tracked X runtime manifest exists, assert it is discovered by
  the real filesystem inventory.
- [x] Run targeted component format, native/wasm clippy, wasm build, and unit
  tests; local-harness runtime format, clippy, and 18 tests; refreshed-CLI
  `firna apps validate apps/x`; and `firna apps package apps/x`.
- [x] Rerun the runtime checks through the tracked canonical-pin manifest and
  lockfile.

## Milestone 6: Add Workspace App Charging

Align the X package with the usage-based app billing contract now available on
the Firna platform. The package remains independently buildable and its agent-
visible success payloads do not expose billing metadata.

- [x] Recheck the merged platform pricing protocol and X's current published
  list rates, then document the retail schedule, caps, failure behavior, daily
  deduplication limitation, and version-bound consent contract.
- [x] Change the manifest to `source.kind: built_in` and declare metered Post
  and User reads plus a capped usage-reported create charge.
- [x] Return the strict priced-result envelope for every successful call while
  preserving stable top-level typed app errors for failed calls.
- [x] Report actual returned Post and expanded User counts, and report $0.015
  for text creation or $0.200 for URL-bearing creation after provider success.
- [x] Add component and platform-runtime tests for exact usage reports, cap
  bounds, zero-result reads, agent-visible usage stripping, and uncharged typed
  failures.
- [x] Update package and runtime documentation, validate the priced built-in
  manifest through the merged platform CLI, and rerun all X package checks.

## Milestone 7: Run the Complete Repository Gate

On 2026-08-04, the complete verifier passed end to end under both the production
Rust 1.89 toolchain and the literal default-toolchain `cargo xtask check`,
including all repository audits, five components, and five platform-runtime
suites.

- [x] Run `cargo fmt --all -- --check`; if it fails, format and rerun it before
  treating any later Rust check as complete.
- [x] Run `cargo xtask rust-file-length-lint --all` if the command is available
  in the pinned workspace, plus the repository's structural audit.
- [x] Run `cargo xtask check` and require a 100% pass rate across existing and X
  package builds, lints, tests, workflow checks, and documentation links.
- [x] Inspect `git diff --check`, the full diff, `git diff --name-status
  origin/main`, and `git diff --diff-filter=D --name-status origin/main`.
- [x] Resolve every compile, lint, test, package, documentation, or smoke-test
  failure before proceeding. If an external service prevents a live check,
  record the exact blocker and all successful local checks.

## Milestone 8: Commit and Push the Checked Work

- [x] Fetch `origin/main`, preserve the pre-integration source tip, audit all
  mainline additions from the merge base, and resolve overlaps path by path.
- [x] Run `git add -A` so every new package, asset, lockfile, protocol document,
  test, and README is tracked.
- [x] Commit the completed implementation with a Conventional Commit title of
  at most 50 characters and a body describing OAuth, cost safety, tests, and
  any approved external setup. A suitable title is
  `feat(x): add read and posting tools`.
- [x] Push the current branch without renaming it.
- [x] Recheck the committed name-status and deletion diff against `origin/main`;
  stop if any mainline feature removal was not explicitly approved.

## Milestone 9: Run Post-Push Review

- [x] After the push, run the explicitly requested `codex-review` workflow so
  `cargo xtask review` evaluates the committed branch against `origin/main` for
  up to ten cycles.
- [x] Independently investigate every review finding, fix every valid finding
  with regression coverage, rerun the relevant checks, commit and push the
  fix, then review again. Record every finding and disposition for the final
  report.
- [x] Map otherwise-unrecognized provider 4xx responses to the stable,
  non-retryable `provider_rejected_request` contract, with component and real
  Wasm-runtime regression coverage for both reads and writes.
- [x] If the final review has no valid findings, report that explicitly and
  proceed to the production-release milestone. Do not complete the plan before
  the production smoke validation is finished.

## Milestone 10: Release and Validate Production and Stable Preview

Keep production and the stable `br-main` preview on separate X OAuth apps while
deploying one immutable package through their normal `main` workflows. Labelled
PR previews continue to exclude X. The user must approve any public write
immediately before it occurs.

- [x] Provision the production client-id and stable-preview client-id/client-
  secret containers, preserve existing versions, and add Terraform imports for
  all four pre-seeded containers.
- [x] Store the corrected production OAuth2 pair in enabled version 2 of its
  containers and disable the versions that briefly assigned that pair to
  preview.
- [ ] Supply a dedicated staging OAuth2 client id and client secret as new
  enabled preview versions; do not use the staging OAuth 1.0 consumer key or
  reuse either production value.
- [x] Bump the package to `1.0.2`, declare `client_id` and `client_secret` as
  required deployment values, and align package tests and documentation.
- [x] Add X only to the stable-main preview allowlist, keep it out of labelled
  PR previews, and reject X preview packages that do not require both isolated
  OAuth values.
- [x] Run the complete `firna-apps` and platform checks, including the platform
  checks required for the affected CLI/app deployment boundary.
- [x] After checks pass, audit both diffs against `origin/main`, stage every
  changed file, commit with Conventional Commits, and push both branches.
- [x] After the `firna-apps` push, run `cargo xtask review` against
  `origin/main`, investigate every finding, and report recommendations without
  automatically changing reviewed code.
- [ ] Merge the `firna-apps` package change first, with required CI and approval,
  so no stable deployment can select the older production-bound manifest.
- [ ] Let the successful app-repository `main` workflow deploy package version
  `1.0.2`; verify the production catalog and existing production OAuth setup.
- [ ] After dedicated staging OAuth2 values are enabled, merge the platform
  change with required CI and approval, apply Terraform so it adopts the
  pre-seeded containers, and let the normal stable-main workflow deploy X to
  `br-main`.
- [ ] Verify `br-main` catalog version `1.0.2`, install it in the nominated
  preview workspace, and complete OAuth with the intended staging X account.
- [ ] Read one known Post and one 10-result recent-search page; verify compact
  outputs, explicit pagination, and the expected usage entries in the X
  Developer Console.
- [ ] Exercise refresh-token rotation without waiting for normal expiry and
  confirm the old access token is not reused or exposed.
- [ ] Ask for explicit approval of the exact smoke-test text and destination X
  account, publish one non-link Post, and verify its returned id and visible
  content. Do not create a second Post to test retry safety.
- [ ] With approval, remove the smoke-test Post manually or through the X UI;
  deletion is not added to the app merely for cleanup.
- [ ] Confirm the spending limit, credit balance, and auto-recharge state after
  the smoke test, then record only redacted usage/cost evidence.
- [ ] Move this plan from Active to Completed in `plans/README.md` after every
  milestone and production validation task is complete.

## Completion Criteria

- The production and stable-preview X developer apps have their exact callbacks
  and isolated credentials outside Git; production retains its approved prepaid
  balance, spending limit, and auto-recharge state.
- Firna refreshes X credentials without exposing tokens and prevents duplicate
  provider writes across durable retries and ambiguous outcomes.
- All three X tools return only real provider data, honor their bounded schemas,
  and are covered by unit, Wasm runtime, error, redaction, and smoke tests.
- All repository checks pass, documentation matches behavior, the committed
  branch is pushed, and every post-push review finding is investigated with
  valid findings fixed and re-reviewed.
- The reviewed package is deployed through the standard production and
  stable-main workflows, then verified in their nominated workspaces. Labelled
  PR previews do not deploy X.
