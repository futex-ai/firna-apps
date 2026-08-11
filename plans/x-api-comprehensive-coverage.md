# Comprehensive X API Coverage

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-11

## Outcome

Expand the first-party X app from four tools to a broad, agent-appropriate
surface for posts, accounts, timelines, engagement, bookmarks, relationships,
lists, Spaces, communities, trends, Direct Messages, and bounded account
actions. Request every OAuth 2.0 scope used by those tools and add a separate
deployment-owned app bearer token for X endpoints that reject user-context
tokens.

Coverage is domain-oriented rather than one tool per provider endpoint. Each
tool uses a closed action or mode enum, issues exactly one bounded provider
request, returns typed compact data, reports exact successful usage, and keeps
provider credentials and raw failures outside model context.

## Deliberate Boundaries

- Include self-serve JSON endpoints with documented authentication, a bounded
  one-request execution model, and a public pricing category.
- Include app-only full-archive search, all-history counts, and location trends
  through an app-owned `bearer_token` secret distinct per deployment class.
- Expand Post creation for quotes, polls, edits, reply settings, Communities,
  and existing media ids without weakening link-cost acknowledgement.
- Include organization-affiliate reads and JSON-only metadata or subtitle
  management for existing media objects.
- Exclude long-running streams, webhooks, account-activity subscriptions,
  compliance jobs, and other control-plane resources from synchronous tools.
- Exclude media upload until the platform attachment bridge can construct X's
  multipart form body without exposing attachment bytes to component memory.
- Exclude Enterprise analytics, encrypted X Chat key management, broadcasts,
  Articles, News, and Community Notes until access and exact provider
  pricing can be verified for the production developer account.
- Never auto-paginate, poll, retry in the component, fan out across connected
  accounts, or combine multiple provider writes in one invocation.

## Milestone 1: Complete the Protocol

Define the complete contract before production code changes. The existing app
remains functional at the end of this milestone.

- [x] Record the official endpoint, authentication, scope, price, pagination,
  and response contracts used by every included mode and action.
- [x] Split the X protocol into focused documents of approximately 250 lines
  while keeping shared authorization, errors, billing, and connection behavior
  canonical in `docs/protocol/x-app.md`.
- [x] Specify closed input schemas, typed outputs, exact provider requests,
  usage units, caps, partial results, and stable validation failures.
- [x] Document the app-only bearer-token boundary and environment-specific
  secret names without recording any credential value.
- [x] Update package, component, runtime, and catalog documentation links.

## Milestone 2: Expand Authorization and Manifest Tools

At the end of this milestone the package manifest validates with the complete
tool catalog and finite charge bounds.

- [x] Bump the X package major version for widened OAuth consent and pricing.
- [x] Add only the OAuth scopes consumed by implemented tools, retaining PKCE,
  refresh rotation, multi-account identity, and per-connection isolation.
- [x] Add the required app-owned bearer-token secret for app-only reads.
- [x] Declare least-privilege HTTP methods, response bounds, every tool schema,
  side-effect class, activity label, auth requirement, and operation name.
- [x] Declare exact metered or usage-reported pricing for every provider-billed
  tool with a finite worst-case hold.
- [x] Extend package tests for scopes, secrets, tools, schemas, pricing, and
  unchanged OAuth lifecycle behavior.

## Milestone 3: Implement Bounded Read Coverage

At the end of this milestone every declared read mode executes through the
opaque host credential boundary and component tests pass.

- [x] Refactor shared DTOs into focused modules before any Rust file reaches
  the 300-line hard limit.
- [x] Implement typed user lookup/search, user feeds, engagement, relationships,
  lists, Spaces, communities, trends, media metadata, and DM reads.
- [x] Implement app-only full-archive search and all-history count reads using
  the app-owned bearer credential without installation credential fallback.
- [x] Preserve request ordering, explicit pagination, empty-result semantics,
  redaction, response caps, and exact returned-resource usage.
- [x] Add component tests for every mode, boundary, usage report, malformed
  response, provider failure, and credential selection.

## Milestone 4: Implement Bounded Account Actions

At the end of this milestone every declared write action dispatches at most one
provider request and ambiguous outcomes fail closed.

- [x] Expand Post creation for quote, poll, edit, reply settings, Community,
  and existing media-id inputs with mutually compatible validation.
- [x] Implement Post, relationship, List, DM, media, and bookmark-folder
  actions with closed enums and action-specific typed bodies.
- [x] Report the documented successful provider cost for each action and no
  usage for typed failures.
- [x] Map transport loss, missing status, 5xx, truncation, and malformed success
  after dispatch to `write_outcome_unknown`.
- [x] Add component tests for exact requests, returned state, pricing, invalid
  field combinations, and one-dispatch write safety.

## Milestone 5: Verify the Real Wasm Boundary

At the end of this milestone the packaged component is exercised through the
pinned platform runtime and all documentation matches behavior.

- [x] Add runtime smoke tests for every tool and representative mode/action,
  including user-token versus app-token routing and usage stripping.
- [x] Add runtime error, scope, response-bound, redaction, OAuth refresh,
  multi-account isolation, and existing-tool regression coverage.
- [x] Update all X READMEs and protocol documents with final behavior, scopes,
  prices, limits, deployment secrets, and deliberate exclusions.
- [x] Run component and runtime formatting, clippy, native tests, locked Wasm
  builds, manifest validation, packaging, and representative tool smokes.

## Milestone 6: Run the Complete Repository Gate

- [x] Run `cargo fmt --all -- --check`, relevant clippy and test suites, and the
  repository Rust file-length/structure audit.
- [x] Run `cargo xtask check` and require a 100% pass rate.
- [x] Resolve every compile, lint, test, package, documentation, and smoke
  failure; record only genuine external provider blockers.
- [x] Inspect `git diff --check`, the complete diff against `origin/main`, and
  all deletions before committing.

## Milestone 7: Commit and Push the Checked Work

- [x] Run `git add -A` so every source, test, protocol, README, manifest, and
  plan change is tracked.
- [x] Commit with a Conventional Commit describing comprehensive X coverage,
  authorization, billing, safety, and verification.
- [x] Push the current branch without renaming it.
- [x] Recheck the committed diff and deletion list against `origin/main`.

## Milestone 8: Run Post-Push Review

- [ ] After push, run `cargo xtask review` against `origin/main`.
- [ ] Investigate every finding without automatically changing reviewed code.
- [ ] Report each finding as a numbered item with severity, feature context,
  impact of doing nothing, lettered solution options, and a recommendation.
- [ ] Evaluate whether a broader rule, test, abstraction, or architectural
  change would prevent the same class of issue more effectively than a direct
  patch.

## Completion Criteria

- Every implemented tool has a closed schema, typed response, exact one-request
  provider contract, finite charge bound, and component plus runtime coverage.
- Every requested OAuth scope and deployment secret is exercised by at least
  one implemented tool; no unused permission is requested.
- Existing X installs upgrade only through explicit version and pricing
  consent, and each connected account remains independently isolated.
- All relevant checks and `cargo xtask check` pass before the complete work is
  committed and pushed; post-push review findings are reported for user choice.
