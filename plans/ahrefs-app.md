# Ahrefs App

Add `apps/ahrefs`: a built-in, explicitly installed Firna app exposing bounded,
synchronous, read-only Ahrefs SEO research tools (site authority, backlinks,
organic search, keyword research, and SERP analysis) backed by each
workspace's own Ahrefs API key.

The package follows the DataForSEO app pattern: one Rust Wasm component, one
provider request per tool invocation, strict typed input schemas, compact
normalized outputs, and no polling handles or deferred operations.

## Product Model

- Ahrefs API v3 is part of a customer's own Ahrefs subscription and is billed
  in API units against that subscription. Sharing one Firna-owned key across
  customer workspaces would resell provider data and concentrate cost, so the
  app uses workspace-supplied credentials, exactly like DataForSEO.
- Install policy is `explicit`. A workspace owner or admin installs the app,
  enters the workspace's Ahrefs API key, and accepts that provider calls are
  billed to the workspace's Ahrefs account.
- Firna verifies the key with the free
  `GET https://api.ahrefs.com/v3/subscription-info/limits-and-usage` endpoint
  (consumes no API units, requires no query parameters), then encrypts and
  stores it. Verification response data is discarded; the app exposes no tool
  that reads account, usage, balance, or pricing data.
- Ahrefs API keys are created under Ahrefs Account settings → API keys by
  workspace owners/admins and expire after one year; the install flow's help
  link points there, and key rotation reuses the same settings entry path.

## Provider Facts (verified 2026-07-23)

- Base URL `https://api.ahrefs.com/v3/`; all tool endpoints are HTTP `GET`
  with query parameters.
- Auth is `Authorization: Bearer <api_key>` plus `Accept: application/json`.
- Table endpoints require a `select` column list; responses are billed by
  rows × fields with a typical minimum of 50 units per request.
- Site Explorer requests against `ahrefs.com` or `wordcount.com` targets are
  free, which gives zero-cost live smoke checks.
- Rate limiting returns HTTP 429 with standard retry/rate-limit headers.

## Platform Dependency (blocking)

The pinned platform revision (`platform.toml`,
`c9553f98b796fb267dd54258c26589a9e7f42811`) cannot collect a
workspace-supplied single API key:

- `fna-apps-interface` manifest validation rejects
  `credential_flows.kind = api_key` as `UnsupportedKind`; only
  `standard_oauth2` and the built-in `basic_auth` flow (DataForSEO) pass.
- There is no settings/install collection path for an `api_key`
  auth requirement.
- Host HTTP already supports the needed injection:
  `credential_injection.kind = "bearer_authorization"` resolves one credential
  reference and injects `Authorization: Bearer <token>`, and the operational
  rate-limit/retry header allowlist already exists for credentialed responses.

Required platform work (in `futex-ai/firna`, mirroring how DataForSEO
introduced the built-in Basic flow):

1. Implement a reviewed built-in `api_key` credential flow: one secret field
   (label, `input_mode = password`, `max_bytes`), `help_url`, and required
   verification metadata. Verification must support success by
   `success_http_status` alone (the Ahrefs verification response has no
   numeric status-code field to select), with 401/403 as conclusive
   rejection and other failures as verifier unavailability.
2. Wire workspace Settings collection, encrypted storage, re-entry, and
   trusted verification for that flow, reusing the bearer injection path for
   tool calls.
3. Add `docs/protocol/ahrefs-app.md` and `docs/protocol/ahrefs-app-tools.md`
   protocol docs (the platform repo owns app protocol docs, as it does for
   DataForSEO).

Fallback considered and rejected: an app-owned `api_key` secret with bearer
injection (the Exa model) works at the current revision but makes Firna the
Ahrefs subscriber for all workspaces, which conflicts with Ahrefs licensing
and unit billing. If product direction changes, Milestones 3–7 still apply
with `secrets: [api_key]`, no auth requirement, and a
`firna-prod-app-ahrefs-api-key` deployment secret.

## Tool Contract (v1)

13 synchronous, read-only tools, all `side_effect: external_read`, all
requiring the `ahrefs_account` auth requirement, each performing exactly one
provider request. Tool limits match DataForSEO: `max_response_bytes` 1 MiB,
`max_component_ms` 300000, manifest `limits.max_tool_response_bytes` 1 MiB.

Shared bounded input rules:

- `target`: string 1..2048 (domain or URL).
- `mode`: enum `exact | prefix | domain | subdomains`.
- `protocol`: enum `both | http | https`.
- `country`: ISO-3166-1 alpha-2, pattern `^[a-z]{2}$`.
- `date`: pattern `^\d{4}-\d{2}-\d{2}$`, required where the provider requires
  a report date (the component has no clock; the model supplies it).
- `keyword`: string 1..250; keyword arrays are unique, bounded lists.
- `limit`: integer 1..50; `offset`: integer 0..1000;
  `timeout_seconds`: integer 1..300.
- `select` column sets and `where`/`having` filters are never model-visible.
  The component sends fixed reviewed column lists and compiles typed inputs
  (for example `dofollow_only`) into fixed filter expressions.

| Tool | Verb | Provider endpoint | Key inputs | Core output |
| --- | --- | --- | --- | --- |
| `ahrefs_domain_rating` | Researching | `site-explorer/domain-rating` | target, date, protocol? | domain rating, Ahrefs rank |
| `ahrefs_backlinks_stats` | Researching | `site-explorer/backlinks-stats` | target, date, mode?, protocol? | live/all-time backlink and refdomain counts |
| `ahrefs_site_metrics` | Researching | `site-explorer/metrics` | target, date, mode?, protocol?, country? | organic/paid keywords, traffic, cost |
| `ahrefs_backlinks` | Researching | `site-explorer/all-backlinks` | target, mode?, aggregation?, dofollow_only?, limit, offset | strongest backlinks with source metrics and anchors |
| `ahrefs_broken_backlinks` | Auditing | `site-explorer/broken-backlinks` | target, mode?, dofollow_only?, limit, offset | broken inbound links with HTTP codes |
| `ahrefs_refdomains` | Researching | `site-explorer/refdomains` | target, mode?, dofollow_only?, limit, offset | referring domains with authority and link counts |
| `ahrefs_anchors` | Analyzing | `site-explorer/anchors` | target, mode?, limit, offset | anchor texts with backlink/refdomain counts |
| `ahrefs_organic_keywords` | Researching | `site-explorer/organic-keywords` | target, country, date, mode?, limit, offset | ranking keywords with position, volume, difficulty, traffic |
| `ahrefs_organic_competitors` | Analyzing | `site-explorer/organic-competitors` | target, country, date, mode?, limit, offset | competing domains with keyword overlap |
| `ahrefs_top_pages` | Researching | `site-explorer/top-pages` | target, country, date, mode?, limit, offset | top pages by organic traffic with top keyword |
| `ahrefs_keyword_overview` | Researching | `keywords-explorer/overview` | keywords (1..100), country | volume, difficulty, CPC, clicks, traffic potential |
| `ahrefs_keyword_ideas` | Researching | `keywords-explorer/matching-terms` \| `related-terms` \| `search-suggestions` via `idea_kind` enum | keywords (1..10 seeds), country, idea_kind, limit, offset | keyword ideas with volume and difficulty |
| `ahrefs_serp_overview` | Searching | `serp-overview/serp-overview` | keyword, country, top_positions? (1..10) | top results with position, DR/UR, backlinks, traffic |

Explicitly excluded from v1 (documented in the app README): history/chart
endpoints, pages-by-* and crawled-pages, outgoing-link endpoints, paid-pages,
volume-history/volume-by-country, Batch Analysis, Rank Tracker, Site Audit,
Brand Radar, Social Media Management, subscription/management endpoints
(verification-only), and any endpoint requiring an Ahrefs project. Extensions
ship as new manifest versions.

Error taxonomy mirrors DataForSEO: typed invalid-input errors before provider
work, `provider_auth_failed` for 401/403, `rate_limited` with
`retry_after_seconds` for 429, `provider_error` with status for other
non-2xx, `provider_unavailable` for transport/host failures, and explicit
truncation handling when `body_truncated` is set.

## Milestones

### Milestone 1: Specification

Complete the implementable contract before any code. Docs-only in this repo;
no `cargo xtask check` required.

- [ ] Pin per-endpoint provider details against the Ahrefs API reference for
      all 13 tools: exact query parameter names, which endpoints require
      `date` versus live/history selectors, history/aggregation enum values,
      keyword list caps, and the fixed `select` column list per tool.
- [ ] Confirm Ahrefs unit-cost and rate-limit response headers, and decide
      which cost metadata the success envelope surfaces given the host
      response header allowlist.
- [ ] Update the tool table and shared input rules in this plan with the
      pinned values, including final JSON Schema bounds per tool.
- [ ] Write the platform handoff spec for the built-in `api_key` credential
      flow (flow shape, verification contract for status-only success,
      settings collection, injection reuse) and file it with the platform
      repo alongside `ahrefs-app.md` / `ahrefs-app-tools.md` protocol drafts.
- [ ] Validate changed Markdown, commit, and push the branch.
- [ ] Run `cargo xtask review` and report findings without auto-fixing.

### Milestone 2: Platform adoption (blocked on `futex-ai/firna`)

Adopt the first platform revision that implements the built-in `api_key`
credential flow. Milestones 3–7 are blocked until this lands.

- [ ] Platform prerequisite merged upstream: `api_key` flow validation,
      collection, verification, and protocol docs (tracked in the platform
      repo, not here).
- [ ] Update `platform.toml` `revision` to the supporting platform commit.
- [ ] Update `fna-apps-interface`/`fna-apps-wasm` git revs in all four
      existing `tests/platform-runtime/Cargo.toml` files and regenerate their
      lockfiles together (the audit rejects partial updates).
- [ ] Update the pinned `firna` CLI install revision in `README.md` and
      anywhere CI installs it; reinstall locally.
- [ ] Run `cargo xtask check` to prove existing apps stay green on the new
      revision.
- [ ] Commit, push, run `cargo xtask review`, and report findings.

### Milestone 3: Package scaffold and manifest

Create `apps/ahrefs` with a complete, validating manifest. The repository
stays green: the audit requires the full package layout, so this milestone
includes compilable component and runtime-test stubs but no tool logic.

- [ ] Scaffold with `firna apps new apps/ahrefs --app-id ahrefs --name Ahrefs
      --non-interactive`; keep the component a standalone Cargo workspace
      with a committed `Cargo.lock`.
- [ ] Add `assets/icon.svg`, `icon.png`, and `icon.png.base64` with a
      `color_pair` distinct from existing apps (proposed primary `#054ADA`
      Ahrefs blue, secondary `#FF8800`; confirm against brand assets).
- [ ] Write `manifest.yaml` version `1.0.0`: `source.kind: built_in`,
      `install.policy: explicit`, `secrets: []`,
      `capabilities.http.allowed_hosts: [api.ahrefs.com]`, empty
      `credential_headers`, `ingress`, `event_subscriptions`, and
      `handler_roles`.
- [ ] Declare the `ahrefs_account` auth requirement (`kind: api_key`,
      `owner: workspace`, `credential_kinds: [api_key]`, `required_for` all
      13 tools) and the `api_key` credential flow with help URL and the free
      `subscription-info/limits-and-usage` verification per the Milestone 1
      spec.
- [ ] Define all 13 tool entries with strict schemas
      (`additionalProperties: false`, bounds from the pinned spec, shared
      YAML anchors for repeated selectors) and DataForSEO-style limits.
- [ ] Add `apps/ahrefs/README.md`, `component/README.md`, and
      `tests/platform-runtime/README.md` following the crate README section
      rules.
- [ ] `firna apps validate apps/ahrefs` passes against the Milestone 2 CLI.
- [ ] Run `cargo xtask check` (updates for xtask app lists land in
      Milestone 6; until then run the component/test builds directly and
      note any gap).
- [ ] Commit, push, run `cargo xtask review`, and report findings.

### Milestone 4: Component implementation

Implement the Wasm component with full unit coverage. All files stay under
the 300-line cap.

- [ ] `src/lib.rs`: wit-bindgen world `firna:app` with `host-http-request`
      import and `call-tool` export, mirroring DataForSEO.
- [ ] `src/ahrefs/` modules: `mod.rs` dispatch, `error.rs` thiserror enum and
      output mapping, `host.rs` `ProviderClient` trait plus
      `WasmProviderClient` sending GET requests with fixed query encoding,
      `credential_injection.kind = "bearer_authorization"`, an
      installation-scoped `api_key` credential reference, and a 1 MiB
      response budget.
- [ ] `src/ahrefs/validation.rs` and `input/` modules: typed
      `deny_unknown_fields` input structs per tool group (overview,
      backlinks, organic, keywords, serp) enforcing every schema bound
      before provider work.
- [ ] `src/ahrefs/tools/` per-group modules: build the fixed `select` lists
      and typed filter expressions, one provider request per invocation.
- [ ] `src/ahrefs/output/` modules: compact normalized success envelope
      (records plus rate-limit/cost metadata per the Milestone 1 decision),
      dropping unrequested provider fields.
- [ ] Unit tests in `_tests_/` with unimock `ProviderClient` fakes: input
      validation, request construction (URL, query, injection, budgets),
      response normalization, error taxonomy, truncation.
- [ ] `cargo fmt`, `cargo clippy --target wasm32-unknown-unknown`,
      `cargo build --target wasm32-unknown-unknown --locked`, and
      `cargo test --locked` all pass for the component manifest.
- [ ] Commit, push, run `cargo xtask review`, and report findings.

### Milestone 5: Platform runtime tests

Prove the packaged component against the pinned platform runtime with a fake
host and no live credentials.

- [ ] `tests/platform-runtime` crate: build/wrap the component via
      `fna-apps-wasm`, following the DataForSEO test crate layout.
- [ ] Package tests: manifest parses and validates via `fna-apps-interface`,
      tool list matches the component dispatch table, every input schema is
      valid JSON Schema, layout/limits assertions.
- [ ] Smoke tests: for each tool, drive `call-tool` through the fake host
      asserting outbound request shape (host, path, query, bearer injection,
      credential scope) and success envelope normalization.
- [ ] Failure tests: 401/403, 429 with retry-after, non-2xx, malformed JSON,
      truncated body, and host credential errors map to the typed error
      contract.
- [ ] `cargo test --manifest-path apps/ahrefs/tests/platform-runtime/Cargo.toml
      --locked` passes.
- [ ] Commit, push, run `cargo xtask review`, and report findings.

### Milestone 6: Repository integration

Make the app a first-class member of repo automation and docs.

- [ ] Add the component and runtime-test manifests to `COMPONENT_MANIFESTS`
      and `RUNTIME_TEST_MANIFESTS` in `xtask/src/check.rs` and update the
      xtask tests that assert those lists.
- [ ] Confirm the repository audit, CI workflow, and deploy planning pick up
      `apps/ahrefs` via discovery (no deployment secret entry is needed for
      the workspace-key model).
- [ ] Update `apps/README.md` (local commands plus Repo-Owned Apps entry) and
      the root `README.md` catalog list.
- [ ] Verify README links against the platform protocol docs landed in
      Milestone 2 and the crate README section requirements.
- [ ] Full `cargo xtask check` passes.
- [ ] Commit, push, run `cargo xtask review`, and report findings.

### Milestone 7: Package validation, live smoke, and release

End-to-end validation of the shippable package.

- [ ] `firna apps validate apps/ahrefs` and `firna apps package apps/ahrefs`
      produce a deterministic vendored source bundle.
- [ ] Live smoke test with a user-supplied Ahrefs API key (prerequisite:
      requires an Ahrefs subscription with API access): verify the
      credential flow end-to-end, then exercise free-tier calls
      (`subscription-info/limits-and-usage`; Site Explorer tools against
      `ahrefs.com`, which consume no units) and record results in the PR.
- [ ] Confirm install/credential UX text matches the product-language rules
      (no internal implementation detail in user-facing copy).
- [ ] Final `cargo xtask check`, commit, push, and `cargo xtask review`;
      report findings.
- [ ] After merge to `main`, confirm the deploy workflow submits `ahrefs`
      1.0.0 and the live catalog shows it; run a post-deploy install check.
- [ ] Move this plan to Completed in `plans/README.md`.

## Risks and Open Questions

- Platform timing: Milestone 2 depends on the `futex-ai/firna` repo shipping
  the `api_key` credential flow; this repo cannot ship the workspace-key
  model without it. The fallback (Firna-owned key) is recorded above but not
  recommended.
- Verification contract: `limits-and-usage` returns subscription metadata.
  The host discards verification responses, but the platform spec must state
  that explicitly so the app can keep its "no account data" stance.
- Key expiry: Ahrefs keys expire after one year; expired keys surface as
  provider auth failures until the workspace re-enters a key. The install
  help copy must mention this.
- Unit costs: even bounded requests bill a ~50-unit minimum per call against
  the workspace's Ahrefs plan; tool descriptions and README must make the
  billing boundary clear, and row limits stay capped at 50.
- Plan availability: Ahrefs API v3 access depends on the workspace's Ahrefs
  plan tier; the README must state the prerequisite rather than implying the
  app grants access.
- Trademark/brand: icon and naming follow the existing provider-app
  convention (DataForSEO, Exa, Slack); confirm Ahrefs brand usage during
  Milestone 3 asset work.
