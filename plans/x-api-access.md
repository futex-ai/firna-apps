# X API Access

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-07-31

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

## Current Constraints

- The X API currently uses prepaid credits and per-resource/per-action charges.
  Rates must be rechecked in the Developer Console immediately before purchase;
  the console is authoritative when it differs from documentation.
- As of 2026-07-31, X documents Post reads at $0.005 per returned resource,
  User reads at $0.010 per returned resource, text Post creation at $0.015 per
  request, and Post creation with a URL at $0.200 per request. Owned Reads are
  $0.001 per resource only when the authenticated X user is also the owner of
  the developer app; that discount must not be promised for other connected
  accounts.
- OAuth scopes are `tweet.read`, `tweet.write`, `users.read`, and
  `offline.access`. The offline scope returns a refresh token; X's user access
  token flow requires refresh-token rotation for a durable connection.
- The Firna platform revision currently pinned by this repository can map and
  store an OAuth refresh token, but it cannot refresh an expired access token.
  This is a release blocker rather than a reason to ship short-lived auth.
- X does not document an idempotency key for `POST /2/tweets`. A durable tool
  retry after an ambiguous provider outcome could therefore publish a duplicate
  unless Firna supplies an at-most-once operation guard.
- Pay-per-use charges accrue to the X developer account that owns the app, not
  to the Firna workspace that authorizes an X account. V1 must launch with a
  console spending limit and bounded tool calls; broader catalog availability
  remains an explicit product/billing decision.

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

- [ ] Add `docs/protocol/x-app.md`, kept below 250 lines, covering manifest
  auth, tool schemas, provider endpoints, bounded outputs, pagination, error
  mapping, credential refresh, operation idempotency, and cost controls.
- [ ] Record the V1 choice of one workspace-owned X account and the three-tool
  surface; do not silently widen the app to per-user grants or extra write
  actions during implementation.
- [ ] Specify the stable behavior for an ambiguous create-Post result: never
  retry automatically, return `write_outcome_unknown`, and direct the caller to
  inspect X before issuing a new operation.
- [ ] Specify how author expansion changes billed resources and ensure it is
  opt-in in both the manifest schema and component request.
- [ ] Confirm whether public catalog availability is acceptable while the X
  developer account pays all usage. If it is not, define and land the required
  catalog allowlist or workspace-private distribution contract before coding.
- [ ] Link the protocol from the root and app documentation where relevant.
- [ ] Validate Markdown links and review the documentation diff.

## Milestone 2: Land Firna Platform Prerequisites

Implement these changes in a separate workspace for the private Firna platform
repository. Do not point this repository at an unmerged or unreviewed platform
revision. The platform must remain green and independently usable after this
milestone.

- [ ] Extend the app protocol's `standard_oauth2` contract with typed access
  token expiry and refresh behavior, including access-token and refresh-token
  credential kinds, `expires_in` mapping, and reuse of the reviewed token URL
  and client-auth method.
- [ ] Persist token expiry metadata without exposing token values; refresh
  proactively near expiry and atomically rotate both tokens when X returns a
  new refresh token.
- [ ] Serialize concurrent refreshes per installation, retry a provider request
  at most once after a successful refresh, and turn terminal `invalid_grant`
  into `auth_required` without leaking the provider response.
- [ ] Add a general at-most-once guard for provider writes that lack provider
  idempotency. Key it by app, installation, tool, and durable `operation_id`;
  persist a successful compact result, and fail closed as
  `write_outcome_unknown` when a claimed operation has an ambiguous outcome.
- [ ] Put refresh, credential vault, operation journal, clock, and provider HTTP
  behavior behind traits and use `unimock` at unit-test boundaries.
- [ ] Add protocol, store, migration, service, runtime, concurrency, redaction,
  and failure-injection tests, including refresh-token rotation and a crash
  immediately before and after provider dispatch.
- [ ] Update all affected Firna protocol docs and crate READMEs, then run the
  platform's complete checks, commit, push, and review workflow.
- [ ] After the platform change lands, update `platform.toml`, the workflow
  pin, every standalone runtime-test manifest, and every lockfile together.
- [ ] Add or update repository-audit tests so a partial platform pin update is
  rejected.

## Milestone 3: Create and Secure the X Developer App

Provision the external app only after the callback and refresh contract are
fixed. At the end of this milestone the X app exists and its credential is in
the production secret store, but no secret appears in Git, shell history,
screenshots, logs, `.context`, or chat.

- [ ] In `console.x.com`, create one production confidential Web App named
  `Firna`, using `https://firna.ai` as the website and plain-language copy that
  says Firna reads and publishes X posts only after workspace authorization.
- [ ] Register the exact production callback
  `https://firna.ai/oauth/x/callback` with no trailing-slash variation.
- [ ] Enable OAuth 2.0 Authorization Code with PKCE. Request scopes from the
  Firna manifest at authorization time rather than enabling unrelated X
  permissions.
- [ ] Capture the public client ID for the manifest and transfer the one-time
  client secret directly into Google Secret Manager as
  `firna-prod-app-x-client-secret`. The human operator performs the secret
  transfer if the available tooling cannot do it without exposing the value.
- [ ] Before purchasing credits, ask the user to approve the exact initial
  credit amount and billing-cycle spending limit. Keep auto-recharge disabled
  unless the user explicitly approves an amount and trigger threshold.
- [ ] Save a redacted record of the app id, callback, app type, approved budget,
  and secret version—not the secret value—in the implementation handoff.
- [ ] Do not add development callbacks to the production app. If a live
  preproduction callback is required, create a separate development app only
  after explicit approval.

## Milestone 4: Build the X App Package

Add a complete standalone package that can build and run through the pinned
Firna Wasm host. All provider data remains live or explicitly unavailable; no
reachable product path contains sample posts or metrics.

- [ ] Create `apps/x/manifest.yaml` with id/name `x`/`X`, version `1.0.0`,
  `source.kind: community`, explicit installation, `api.x.com` as the only
  HTTP host, the public client ID, the app-owned `client_secret` declaration,
  and workspace-owned OAuth using `client_secret_basic` plus required S256
  PKCE.
- [ ] Map `access_token`, `refresh_token`, granted scopes, and expiry through
  the platform's reviewed refresh contract. Store and inject tokens only by
  opaque credential reference.
- [ ] Add an official X brand asset as SVG source, a PNG catalog icon under the
  manifest size limit, matching base64 source, and a legible color pair. Do not
  generate or redraw the X trademark.
- [ ] Create the standalone Rust component and lockfile. Keep module roots thin,
  production and test files under 300 lines, public APIs documented, imports
  ordered, and all impure HTTP/runtime behavior behind trait objects.
- [ ] Model requests, successful responses, pagination metadata, and provider
  errors with typed structs/enums. Do not retain or pass through arbitrary
  provider JSON beyond the HTTP boundary.
- [ ] Implement the three V1 tools with strict input validation, bounded
  provider response reads, explicit pagination, compact outputs, stable error
  mapping, rate-limit metadata, and no implicit retries.
- [ ] Apply the at-most-once operation guard to `x_create_post` before provider
  dispatch and return a stored success for a repeated completed operation.
- [ ] Add source-adjacent unit tests under `_tests_` using `unimock` for HTTP and
  operation-guard boundaries. Cover malformed calls, URL acknowledgement,
  provider errors, truncation, pagination, auth loss, and repeated writes.
- [ ] Add package, component, and platform-runtime READMEs with the required
  responsibilities, quick-start commands, key code, related docs, tool table,
  OAuth setup, cost behavior, secret name, and non-goals.
- [ ] Update the root and `apps/README.md` catalog summaries and local command
  examples without turning them into exhaustive feature lists.

## Milestone 5: Add Runtime and Repository Verification

Exercise the real built Wasm component through the pinned platform host. The
new package and all existing packages must be green together.

- [ ] Add `apps/x/tests/platform-runtime` as a standalone test crate with its
  own lockfile and publishable-quality README.
- [ ] Validate the manifest id, version, icon, host allowlist, explicit install,
  OAuth owner/scopes/client method/PKCE/refresh mapping, tool names, schemas,
  side effects, and response limits.
- [ ] Smoke every tool through the real Wasm component and a `WasmHostMock`;
  assert exact method, URL, query/body shape, opaque credential scope, output,
  and the absence of null or undeclared fields.
- [ ] Test missing credentials, expired-token refresh, rotated refresh tokens,
  missing scopes, 401/403/404/429/credit exhaustion/5xx responses, malformed or
  truncated JSON, and redaction of provider details.
- [ ] Prove a repeated completed create operation returns its journaled result
  without a second X request and an ambiguous claimed operation never sends a
  second Post.
- [ ] Add both X manifests to `xtask/src/check.rs` and strengthen its inventory
  test so every discovered component and runtime-test manifest is covered,
  preventing future packages from being omitted by a hard-coded list.
- [ ] Run targeted component format, clippy, build, and unit tests; runtime
  format, clippy, and tests; `firna apps validate apps/x`; and
  `firna apps package apps/x`.

## Milestone 6: Perform an Authorized Live Smoke Test

Verify the real provider boundary without broadening the release. The user must
approve the public write immediately before it occurs.

- [ ] Deploy the reviewed X package and app secret to the intended environment,
  install it in the nominated smoke-test workspace, and complete OAuth with the
  intended X account.
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

## Milestone 7: Run the Complete Repository Gate

- [ ] Run `cargo fmt --all -- --check`; if it fails, format and rerun it before
  treating any later Rust check as complete.
- [ ] Run `cargo xtask rust-file-length-lint --all` if the command is available
  in the pinned workspace, plus the repository's structural audit.
- [ ] Run `cargo xtask check` and require a 100% pass rate across existing and X
  package builds, lints, tests, workflow checks, and documentation links.
- [ ] Inspect `git diff --check`, the full diff, `git diff --name-status
  origin/main`, and `git diff --diff-filter=D --name-status origin/main`.
- [ ] Resolve every compile, lint, test, package, documentation, or smoke-test
  failure before proceeding. If an external service prevents a live check,
  record the exact blocker and all successful local checks.

## Milestone 8: Commit and Push the Checked Work

- [ ] Fetch `origin/main`, preserve the pre-integration source tip, audit all
  mainline additions from the merge base, and resolve overlaps path by path.
- [ ] Run `git add -A` so every new package, asset, lockfile, protocol document,
  test, and README is tracked.
- [ ] Commit the completed implementation with a Conventional Commit title of
  at most 50 characters and a body describing OAuth, cost safety, tests, and
  any approved external setup. A suitable title is
  `feat(x): add read and posting tools`.
- [ ] Push the current branch without renaming it.
- [ ] Recheck the committed name-status and deletion diff against `origin/main`;
  stop if any mainline feature removal was not explicitly approved.

## Milestone 9: Run Post-Push Review

- [ ] After the push, run `cargo xtask review` so the reviewer evaluates the
  committed branch against `origin/main`.
- [ ] Do not automatically fix review findings in this change. Report every
  finding as a numbered item with severity, codebase and feature context,
  impact of doing nothing, lettered solution options, and a recommended option.
- [ ] If the review has no findings, report that explicitly and move this plan
  from Active to Completed in `plans/README.md` only after every preceding TODO
  is checked.

## Completion Criteria

- The X developer app exists with the exact callback, confidential credentials
  stored outside Git, an approved prepaid balance, a spending limit, and the
  approved auto-recharge state.
- Firna refreshes X credentials without exposing tokens and prevents duplicate
  provider writes across durable retries and ambiguous outcomes.
- All three X tools return only real provider data, honor their bounded schemas,
  and are covered by unit, Wasm runtime, error, redaction, and smoke tests.
- All repository checks pass, documentation matches behavior, the committed
  branch is pushed, and the post-push review is reported without silently
  applying findings.
