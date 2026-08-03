# Browser Use Billable App

Add [browser-use.com](https://browser-use.com/) as a first-party Firna app
package (`apps/browseruse`) that lets agents run cloud browser-automation
tasks, with usage billed to the workspace credit wallet through the platform's
app usage pricing contract.

## Goal

- Agents can start a Browser Use Cloud run, poll it, and cancel it through
  manifest-declared tools.
- The workspace is charged the run's real provider cost (plus a configured
  markup) through the platform wallet — no bring-your-own-key required. Firna
  owns the provider API key as an app-owned secret.

## Platform Contract (already shipped at the pinned revision)

The pinned platform revision in [`platform.toml`](../platform.toml) fully
supports priced apps. Key references:

- [App Usage Pricing](https://github.com/futex-ai/firna/blob/main/docs/protocol/app-usage-pricing.md):
  manifest `pricing` block, hold→settle flow, task-scoped charging, the
  `{"output": ..., "usage": ..., "task_events": ...}` priced-result envelope,
  and the `usage_reporter_export` sweep contract.
- [Apps protocol](https://github.com/futex-ai/firna/blob/main/docs/protocol/apps.md):
  manifest rules, `source.kind: built_in`, install policies, capabilities.
- Policy ceilings (`billing-commercial-and-risk-policy.md`): worst-case own
  charge per call ≤ $10, task cap ≤ $50, task duration 60 s – 24 h. Priced
  manifests must be `built_in` and must not be `workspace_default`.
- `fna-apps-interface` types: `AppToolResult.task_events`,
  `AppTaskLifecycleReport::{TaskOpened, TaskTerminal}`,
  `AppTaskUsageReport::ReportedCost`, `AppTaskUsageReporterRequest/Result`.
- `fna-apps-wasm` invokes the manifest `usage_reporter_export` as an extra
  component export with the same JSON-string ABI as `call-tool`.

Platform-side deployment must have `billing.enabled` and
`billing.app_charging_enabled`; priced tools otherwise fail closed with
`app_charge_unavailable`. That flag work is out of scope for this repository.

## Provider Contract (Browser Use Cloud API v4)

- Base `https://api.browser-use.com/api/v4`, auth header
  `X-Browser-Use-API-Key` (app-owned secret).
- `POST /runs`: `task` (required), `model` (optional; provider default
  `minimax-m3`), `sessionId` (optional follow-up in one session),
  `maxCostUsd` (provider-side cost cap), `browserSettings`.
- `GET /runs/{run_id}`: `status`
  (`queued|dispatching|running|completed|failed|cancelled`), `result`,
  `error`, `totalInputTokens`, `totalOutputTokens`, `totalCostUsd`
  (decimal string USD), `sessionId`.
- `GET /runs/{run_id}/status`: cheap status-only poll.
- `POST /runs/{run_id}/cancel`: returns the full run summary, so cancel can
  settle in one call when the returned status is terminal.
- No idempotency key or client-reference field exists on `POST /runs`.

## Design

### Package

`apps/browseruse` following the `apps/exa` / `apps/dataforseo` conventions:

- `manifest.yaml`: id `browseruse`, name `Browser Use`, version `1.0.0`,
  `source.kind: built_in`, `install.policy: explicit`, PNG icon with
  `color_pair`, secret `api_key` (required; production Secret Manager id
  `firna-prod-app-browseruse-api-key`), `capabilities.http.allowed_hosts:
  [api.browser-use.com]` with `credential_headers: [x-browser-use-api-key]`,
  no ingress/events, `env.price_multiplier_bp` (markup in basis points).
- `component/`: standalone Cargo workspace Wasm component with WIT exports
  `call-tool` and `report-task-usage`, and import `host-http-request`.
- `tests/platform-runtime/`: integration tests pinned to the platform
  revision, like the existing apps.

### Tools

| Tool | Operation | Side effect | Activity label |
| --- | --- | --- | --- |
| `browseruse_run_task` | `browseruse.run_task` → `POST /runs` | `external_write` | `Starting browser task` |
| `browseruse_get_run` | `browseruse.get_run` → `GET /runs/{id}` | `external_read` | `Checking browser task` |
| `browseruse_cancel_run` | `browseruse.cancel_run` → `POST /runs/{id}/cancel` | `external_write` | `Cancelling browser task` |

Inputs: `browseruse_run_task` takes `task` (required), optional `model`
(free-form string; known models listed in the description so new provider
models need no manifest release), optional `session_id`, optional
`max_cost_usd` (agent-requested bound, clamped to the manifest cap).
`browseruse_get_run` / `browseruse_cancel_run` take `run_id`.

Outputs return run id, status, session id, and result/error text. Tool
outputs never include provider cost values: financial data reaches owners and
admins through Billing/Usage surfaces only, never agent-visible payloads.

### Pricing block

```yaml
pricing:
  tools:
    - tool: browseruse_run_task
      opens_task: browseruse_run
    - tool: browseruse_get_run
      kind: usage_reported
      max_cost_usd_micros_per_call: 1000
    - tool: browseruse_cancel_run
      kind: usage_reported
      max_cost_usd_micros_per_call: 1000
  tasks:
    - task: browseruse_run
      kind: usage_reported
      max_cost_usd_micros_per_task: 10000000
      max_task_duration_seconds: 7200
      usage_reporter_export: report-task-usage
```

- `browseruse_run_task` carries no own price; opening reserves the $10.00
  task cap. The component starts the provider run with `maxCostUsd` derived
  from the cap and multiplier, and reports `task_opened` with the provider
  run id.
- `browseruse_get_run` and `browseruse_cancel_run` are declared
  `usage_reported` with a $0.001 cap and always report
  `{"kind": "reported_cost", "cost_usd_micros": 0}` as their own usage, so
  they settle at zero. The declaration exists because the host only decodes
  the priced-result envelope — and therefore `task_events` — for tools with a
  `pricing.tools` entry. This is what lets a poll or cancel result carry
  `task_terminal` and settle the task promptly instead of waiting for the
  post-deadline sweep.
- When a poll/cancel observes a terminal provider status, it reports
  `task_terminal` with `reported_cost` = provider `totalCostUsd` converted to
  integer micro-USD (ceiling), multiplied by `price_multiplier_bp`.
- `max_task_duration_seconds: 7200` covers the provider's default 60-minute
  browser session plus queue/dispatch margin; the platform sweep reconciles
  abandoned holds after that deadline via `report-task-usage`.

### `report-task-usage` export

- `ProviderTask` lookup → `GET /runs/{id}`: HTTP 404 → `absent`; terminal
  status → `terminal` with reported cost; otherwise `running`.
- `OpeningOperation` lookup → `absent` (see design decision 2).

### Money handling

All arithmetic is integer-only (no floats): parse the provider's decimal USD
string into micro-USD with ceiling rounding, apply
`customer = ceil(provider × bp / 10000)`, clamp at the manifest cap. The
provider-side `maxCostUsd` is `floor(cap × 10000 / bp)` rendered as a decimal
string so an in-bounds provider cost can never exceed the customer cap after
markup. The host additionally clamps and marks `capped` on over-cap reports.

## Design Decisions (recommendations)

1. **Prompt settlement via zero-priced poll tools** (chosen above). The
   platform's own pricing example leaves poll tools unpriced, which would
   hold the full task reserve until the post-deadline sweep (~2 h) even for a
   2-minute run. The zero-reporting `usage_reported` declaration keeps
   polling effectively free while releasing the customer's hold as soon as a
   terminal status is observed. Alternative: sweep-only settlement (simpler
   manifest, worse credit UX). Recommended: as specified.
2. **`OpeningOperation` reporter lookup returns `absent`.** Browser Use v4
   has no client-reference on run creation, so a crash-orphaned run cannot be
   found by Firna operation id. Returning `absent` releases the customer's
   reserve; per policy, provider cost for uncharged failures is borne by the
   app and is bounded by the provider-side `maxCostUsd`. Alternatives:
   (a) return an error so the hold lands in operator reconciliation after
   24 h of retries — honest but locks customer credit and creates operator
   noise for a rare crash window; (b) create a named provider workspace per
   run to make runs discoverable — provable absence, but adds a provider call
   to every run and depends on undocumented workspace limits. Recommended:
   `absent`, documented in the app README, revisit if Browser Use adds client
   references.
3. **Charge terminal runs regardless of provider status.** `completed`,
   `failed`, and `cancelled` runs all report their real `totalCostUsd`:
   tokens and browser time were genuinely consumed, matching how model calls
   bill regardless of answer quality. Alternative: zero-report failed runs
   (Firna absorbs; invites free-retry abuse). Recommended: charge actuals.
4. **Markup via `env.price_multiplier_bp`, initial value `12000` (1.2×).**
   The run's `totalCostUsd` covers provider-marked-up tokens but browser
   hosting ($0.02/h) and proxy bandwidth bill on the provider session, not
   the run; the markup absorbs those plus margin. A different value ships as
   a new app version. Alternative: 1:1 pass-through (simpler copy, negative
   margin). Recommended: 1.2×.
5. **Task cap $10.00, duration 7200 s.** Well inside policy ceilings; typical
   runs on the default model cost cents, and the cap is also the per-run
   credit reserve, so $50 would block low-credit workspaces. Raising it later
   is a new version plus fresh install consent.

## Out Of Scope

- Platform-side product surfaces (catalog pricing display, install consent,
  billing views) — already generic in the platform repository. No UI or
  mockup work exists in this repository, so no `ui`/`mockup` milestones.
- Provisioning the production Browser Use API key (operator step in Google
  Secret Manager) and enabling `billing.app_charging_enabled`.
- v2 ideas: file attachments/workspaces, structured-output helper, browser
  profiles, session queueing, run events streaming.

## Milestones

### Milestone 1: Package scaffold and manifest

The installable package skeleton: manifest with pricing, icon, component that
compiles with both exports and returns typed not-implemented errors, READMEs,
and repository registration. Repo stays green throughout.

- [ ] Create `apps/browseruse/manifest.yaml` per the design (id, icon,
      `built_in`, `explicit`, secret, capabilities, env, tools with input
      schemas and activity labels, pricing block, empty ingress/events)
- [ ] Add `assets/` icon: PNG ≤ 64 KiB ≤ 1024 px plus editable SVG source,
      with a `color_pair` distinct from existing apps at small sizes
- [ ] Scaffold `component/` as a standalone Cargo workspace (locked deps,
      wasm32-unknown-unknown target) with WIT world exporting `call-tool`
      and `report-task-usage`, importing `host-http-request`
- [ ] Implement tool dispatch skeleton returning typed
      `{"ok": false, "error": ...}` payloads; unit test the dispatch
- [ ] Write `apps/browseruse/README.md` and `component/README.md` following
      the crate README section rules
- [ ] Register `apps/browseruse/component/Cargo.toml` in
      `xtask/src/check.rs` `COMPONENT_MANIFESTS` and update
      `xtask/src/_tests_/check_tests.rs`
- [ ] Update `apps/README.md` (Repo-Owned Apps, Local Commands) and the
      workspace `README.md` catalog list
- [ ] Run `cargo xtask check`; fix until green
- [ ] Commit with Conventional Commits, push the branch
- [ ] Run `cargo xtask review`; report findings without auto-fixing

### Milestone 2: Tools, billing envelope, and usage reporter

The full component implementation with complete unit-test coverage using the
mocked host (no real network).

- [ ] Provider types module: run create/summary DTOs, status enum, typed
      error enum per the error-handling standards
- [ ] Money module: decimal-USD-string → micro-USD ceiling parser, basis-
      point markup, cap clamp, provider `maxCostUsd` derivation; exhaustive
      unit tests (rounding, huge values, malformed strings, zero)
- [ ] `browseruse_run_task`: `POST /runs` with credential injection and
      derived `maxCostUsd`; priced envelope output with `task_opened`;
      input validation
- [ ] `browseruse_get_run`: status mapping; zero own-usage report on every
      call; `task_terminal` with marked-up reported cost when terminal
- [ ] `browseruse_cancel_run`: cancel → returned summary; settle inline when
      the response is terminal, otherwise leave settlement to a later poll
- [ ] `report-task-usage` export: `ProviderTask` lookup via
      `GET /runs/{id}` (404 → absent, terminal → usage, else running);
      `OpeningOperation` → absent per design decision 2
- [ ] Provider error mapping: 401/403/5xx → `provider_unavailable`, 429 →
      `rate_limited` with `retry-after`, 402 → typed provider-credit error,
      422 → typed invalid-input error; never stringly-matched
- [ ] Unit tests for every tool and the reporter over the mocked host,
      including envelope shape, task events, and cost math in Firna's favor
- [ ] Keep every Rust file ≤ 300 lines; module-level docs and public-item
      doc comments throughout
- [ ] Run `cargo xtask check`; fix until green
- [ ] Commit with Conventional Commits, push the branch
- [ ] Run `cargo xtask review`; report findings without auto-fixing

### Milestone 3: Platform-runtime integration tests

Prove the real Wasm component against the pinned platform runtime, matching
the existing apps' test layout.

- [ ] Create `apps/browseruse/tests/platform-runtime/` crate pinned to the
      `platform.toml` revision (`fna-apps-interface`, `fna-apps-wasm`)
- [ ] Package tests: manifest parses and validates through
      `fna-apps-interface`, including the pricing block, `built_in` source,
      explicit install, secret and capability declarations
- [ ] Smoke tests through `WasmComponentRuntime` with mocked host HTTP:
      run/get/cancel round trips, priced envelope decoding, `task_opened`
      and `task_terminal` propagation, zero own-usage settlement
- [ ] Reporter tests: invoke `report-task-usage` through the runtime for
      both lookup kinds and all three result states
- [ ] Target-directory and support modules mirroring the exa test crate
- [ ] Register the test manifest in `xtask/src/check.rs`
      `RUNTIME_TEST_MANIFESTS` and update `check_tests.rs`
- [ ] Run `cargo xtask check`; fix until green
- [ ] Commit with Conventional Commits, push the branch
- [ ] Run `cargo xtask review`; report findings without auto-fixing

### Milestone 4: Docs and deployment readiness

Ship-ready documentation and deployment verification.

- [ ] Finalize `apps/browseruse/README.md`: tool reference, billing
      behavior (cap, markup, terminal-status charging, orphan policy),
      install flow, and the `firna apps validate apps/browseruse` command
- [ ] Document the operator step: provision
      `firna-prod-app-browseruse-api-key` in Google Secret Manager with a
      funded Browser Use Cloud key before promoting the version, and the
      platform `billing.app_charging_enabled` dependency
- [ ] Confirm `scripts/plan-app-deploys.py` and the deploy workflow pick up
      the new app without changes (workflow test script passes)
- [ ] Re-check README/docs consistency across workspace, `apps/`, app, and
      component READMEs
- [ ] Run `cargo xtask check`; fix until green
- [ ] Commit with Conventional Commits, push the branch
- [ ] Run `cargo xtask review`; report findings without auto-fixing

## Open Questions For Review

1. Confirm the $10.00 task cap and 1.2× markup launch values (design
   decisions 4–5).
2. Confirm charging `failed`/`cancelled` runs at actual cost (decision 3).
3. Confirm the `absent` orphan-lookup policy versus operator reconciliation
   (decision 2); optionally raise a client-reference feature request with
   Browser Use.
4. Should a platform protocol note be added (platform repo) documenting the
   zero-priced-poll settlement pattern so future priced apps reuse it?
