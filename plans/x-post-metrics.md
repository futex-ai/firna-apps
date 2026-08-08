# X Post Metrics

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-08

## Outcome

Add a bounded `x_get_post_metrics` tool to the first-party X app so an
authorized workspace can inspect current engagement metrics for explicitly
selected Posts. The tool returns real X data for impressions, likes, replies,
reposts, quotes, and bookmarks, with an opt-in owned-Post view for engagements,
URL clicks, and profile clicks when X makes those private fields available.

This change uses the existing workspace OAuth authorization and the standard
pay-per-use Post lookup endpoint. It does not claim to provide total account
profile views: X documents `user_profile_clicks` attributed to a Post, which is
not the same metric. Enterprise Post time-series analytics, media analytics,
Ads API reporting, scheduled collection, and dashboard storage remain separate
future changes.

## Product and Technical Contract

- Add one tool named `x_get_post_metrics`; do not silently widen the compact
  output of `x_get_posts` or `x_search_recent_posts`.
- Accept 1-10 unique decimal Post ids and an optional
  `include_private_metrics` boolean that defaults to `false`.
- Issue at most one explicit `GET https://api.x.com/2/tweets` provider request
  per invocation. Do not paginate, poll, retry in the component, or synthesize
  a time series from snapshots.
- Always request typed public Post metrics: impressions, likes, replies,
  reposts, quotes, and bookmarks.
- When private metrics are explicitly requested, request only the documented
  owned-Post fields needed for engagements, URL clicks, and profile clicks.
  Private metrics are available only under X's user-context and retention
  rules; omitted provider fields remain unavailable and must never become
  invented zeroes.
- Name the attributed metric `profile_clicks`, explain that it means clicks
  from a Post to the author's profile, and never label it `profile_views`.
- Preserve partial-success behavior for missing Post ids and define a stable,
  typed representation for Posts whose private metrics are unavailable without
  guessing whether ownership, age, or provider policy caused the omission.
- Keep `tweet.read`, `tweet.write`, `users.read`, and `offline.access` as the
  OAuth scope set unless live provider documentation proves an additional
  scope is required. Any scope expansion requires an explicit protocol and
  consent decision before implementation continues.
- Keep the dedicated `GET /2/tweets/analytics` and
  `GET /2/media/analytics` endpoints out of this change because X currently
  documents them as Enterprise features. Do not fall back to Ads API endpoints
  or add `ads-api.x.com` to the host allowlist.
- Recheck the exact provider price and billing unit in the X Developer Console
  before fixing the manifest price. The tool must declare a finite worst-case
  wallet hold, charge only validated successful resources, and remain
  uncharged on handled failure.
- Bump the X package version for the new tool and immutable price contract; an
  installed workspace must explicitly consent to the updated package version.

## Milestone 1: Complete the Analytics Protocol

Document the contract before production code changes. The existing three-tool
app remains fully functional at the end of this milestone.

- [ ] Recheck X's official Post metrics, authentication, retention, pricing,
  and Enterprise-entitlement documentation, recording the verification date
  and treating the Developer Console as authoritative for current prices.
- [ ] Add a focused `docs/protocol/x-app-metrics.md` so the existing X protocol
  remains below its approximate 250-line limit; link both documents without
  duplicating or contradicting the shared OAuth, error, and billing contract.
- [ ] Specify the exact input schema, provider query, public and private metric
  structs, output ordering, partial results, missing ids, and unavailable-field
  representation.
- [ ] Specify stable validation and provider-error behavior, including private
  metric omission, malformed metric objects, oversized responses, rate limits,
  budget exhaustion, and authentication loss without exposing provider text.
- [ ] Specify the verified usage meter, unit price, per-call cap, successful
  settlement rules, and behavior if X's console price differs from the public
  documentation or the existing `post_read` unit.
- [ ] Document that profile clicks are Post-attributed and that total profile
  views, historical time series, media analytics, promoted/Ads analytics, and
  automatic collection are unavailable through this tool.
- [ ] Update the X package README and relevant protocol links only as needed to
  make the planned boundary discoverable, then validate Markdown links and
  inspect the documentation diff.

## Milestone 2: Add the Manifest and Component Tool

Implement the documented tool behind the existing opaque credential boundary.
At the end of this milestone the X component builds and all component tests
pass with the new tool available.

- [ ] Add failing component tests for the complete public-metric result,
  opt-in private fields, provider-omitted private fields, genuine zero values,
  partial Post results, malformed metrics, and exact metered usage before
  implementing the behavior.
- [ ] Bump the package version and add the `x_get_post_metrics` manifest schema,
  side-effect classification, OAuth requirements, response limit, timeout,
  activity label, and verified bounded pricing declaration.
- [ ] Add fully typed request, provider-response, output, and error models; do
  not pass through arbitrary JSON or expose fields outside the protocol.
- [ ] Implement strict input validation and one bounded Post lookup using the
  existing opaque workspace access-token reference.
- [ ] Map public and optional private metrics without treating omission as zero,
  leaking provider data, or changing the existing compact Post tools.
- [ ] Report usage only after a validated provider success, count only returned
  billable resources, and preserve uncharged typed failures.
- [ ] Keep changed Rust files below 300 lines, split growing modules through the
  normal module system, and update module/public API documentation.
- [ ] Run component format, native unit tests, Wasm clippy, and the locked
  `wasm32-unknown-unknown` build until they pass with no warnings or failures.

## Milestone 3: Verify the Runtime Boundary and Documentation

Exercise the packaged component through the pinned Firna runtime. At the end
of this milestone every tool remains locally usable and documented.

- [ ] Add platform-runtime tests for manifest discovery, exact tool schema,
  existing OAuth scopes, opaque credential injection, request parameters,
  public/private results, omissions versus zeroes, partial results, response
  bounds, typed errors, usage stripping, and wallet settlement.
- [ ] Prove that each invocation emits at most one provider request and that no
  existing read or write tool changes its request or output contract.
- [ ] Add regressions for missing authorization, missing scopes, provider 4xx,
  429, budget exhaustion, 5xx, malformed/truncated JSON, and redaction of raw
  provider bodies and credentials.
- [ ] Update `apps/x/README.md`, the component/runtime READMEs, app catalog
  documentation, and the analytics protocol with the final implemented
  behavior, price, limits, and non-goals.
- [ ] Run the X component and platform-runtime format, clippy, build, and test
  commands with a 100% pass rate.
- [ ] Run `firna apps validate apps/x` and `firna apps package apps/x`, then
  smoke the tool through the real Wasm component with public metrics, private
  metrics, and an unavailable-private-metrics response.
- [ ] If the pinned platform cannot express the documented typed result or
  bounded charge, stop and add a new platform-prerequisite milestone followed
  by a replacement component milestone; do not mix platform work into the
  current app milestone.

## Milestone 4: Run the Complete Repository Gate

Require the repository to remain green before committing the feature.

- [ ] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
  rerun the check before proceeding.
- [ ] Run `cargo xtask rust-file-length-lint --all` and the repository
  structural audit.
- [ ] Run every relevant targeted test discovered during implementation, then
  run `cargo xtask check` and require a 100% pass rate.
- [ ] Resolve every compile, lint, test, package, documentation, or local smoke
  failure. If an external provider blocks live verification, record the exact
  blocker and all checks that succeeded.
- [ ] Fetch `origin/main`, capture the source tip before integration, audit
  mainline additions from the merge base, and resolve any overlap path by path
  without deleting or overriding mainline behavior.
- [ ] Inspect `git diff --check`, the complete diff, `git diff --name-status
  origin/main`, and `git diff --diff-filter=D --name-status origin/main`; stop
  unless every deletion is explicitly authorized.

## Milestone 5: Commit and Push the Checked Work

- [ ] Run `git add -A` so the manifest, component, tests, protocol, READMEs,
  lockfiles, and plan updates are all tracked.
- [ ] Commit the completed feature using a Conventional Commit title no longer
  than 50 characters and a body describing the metric boundary, pricing,
  privacy behavior, tests, and non-goals. A suitable title is
  `feat(x): add post metrics tool`.
- [ ] Push the current branch without renaming it.
- [ ] Inspect `git diff --name-status origin/main..HEAD` and the deletion diff
  after the commit; stop if an unauthorized mainline removal is present.

## Milestone 6: Run Post-Push Review

- [ ] After the push, run `cargo xtask review` so the AI reviewer evaluates the
  committed local diff against `origin/main`.
- [ ] Independently investigate every finding, but do not automatically change
  reviewed code. Report each finding as a numbered item with severity,
  codebase and feature context, the impact of doing nothing, lettered solution
  options, and a clearly recommended option.
- [ ] For each recommendation, assess whether a direct fix is sufficient or a
  broader test, lint, rule, abstraction, or architectural change would better
  prevent the same class of defect.
- [ ] Wait for the user's decision on review findings. If fixes are approved,
  add a new milestone rather than reopening a completed milestone, implement
  regression coverage, rerun the relevant gates, commit and push, and rerun
  review.

## Milestone 7: Release and Live Read Smoke Test

Release through the normal `main` workflows only. At the end of this milestone
the catalog version is deployed and verified against real provider data.

- [ ] Merge only after required CI and approval; do not deploy the feature
  package directly from the workspace branch.
- [ ] Verify the new X catalog version in production and stable `br-main`
  preview after their normal deployment workflows complete.
- [ ] In the nominated preview workspace, connect the intended X test account
  and select a known public Post plus a recent Post owned by that account; do
  not record credentials or private provider payloads in the handoff.
- [ ] Invoke one bounded public-metrics read and confirm impressions and
  engagement counts match the visible/provider response without invented data.
- [ ] Invoke one explicitly approved private-metrics read for the owned Post and
  verify available engagements, URL clicks, and profile clicks, or verify the
  documented unavailable state if X omits them.
- [ ] Confirm the Developer Console usage, Firna wallet charge, credit balance,
  spending limit, and auto-recharge state match the reviewed price contract;
  retain only redacted evidence.
- [ ] Move this plan from Active to Completed in `plans/README.md` only after
  every milestone, live validation task, and approved review-fix milestone is
  complete.

## Completion Criteria

- `x_get_post_metrics` returns bounded, typed, real public metrics and opt-in
  owned-Post private metrics without changing the three existing tools.
- Missing, unavailable, and genuine zero values remain distinguishable; Post-
  attributed profile clicks are never presented as total profile views.
- OAuth scopes, pricing, wallet holds, usage settlement, response limits, and
  provider failures match the completed protocol and current X console.
- Component, Wasm-runtime, package, smoke, and complete repository checks pass;
  all changes are committed and pushed before post-push review.
- Every review finding is investigated and presented for user decision, and the
  normally deployed package is verified with redacted live read evidence.
