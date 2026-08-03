# Xero Accounting App

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-03

## Outcome

Add `apps/xero`, an explicitly installed first-party app that lets a Firna
workspace connect one Xero organisation and inspect its core accounting data.
V1 is deliberately read-only: agents can retrieve organisation settings,
accounts, contacts, sales invoices, bills, payments, bank transactions, and
three financial reports, but cannot create, approve, send, pay, void, archive,
or otherwise mutate Xero records.

The integration uses Xero's confidential OAuth 2.0 authorization-code flow,
keeps access and rotating refresh tokens behind Firna's opaque credential
boundary, and binds every Accounting API request to the organisation selected
by a workspace administrator. Creating the Xero developer app, transferring
its secret, selecting the production connection tier, and completing a live
Demo Company smoke test are external actions requiring a human operator.

## V1 Product Contract

- One active Xero organisation per Firna workspace. OAuth may expose several
  authorised Xero connections, but an admin explicitly selects one and that
  `tenantId` becomes the installation's immutable provider-account binding.
- The selected tenant id is never a model-visible tool input. Trusted host code
  injects it as `xero-tenant-id`; the component cannot override the header or
  use an arbitrary organisation id.
- Install policy is `explicit`. The authorising Xero user must be able to grant
  access to the selected organisation, and some reports additionally require
  the corresponding Xero user role.
- OAuth uses a confidential Xero Web App with `client_secret_basic`; the
  provider's separate public-client PKCE app type is not used.
- V1 requests only `offline_access` and granular read scopes:
  `accounting.settings.read`, `accounting.contacts.read`,
  `accounting.invoices.read`, `accounting.payments.read`,
  `accounting.banktransactions.read`,
  `accounting.reports.profitandloss.read`,
  `accounting.reports.balancesheet.read`, and
  `accounting.reports.trialbalance.read`.
- Every tool performs exactly one Accounting API request. Collection tools
  expose one bounded page and never drain subsequent pages automatically.
- List inputs use typed filters only. Raw Xero `where` and `order` expressions
  are not model-visible because they would bypass schema bounds and efficient
  query rules.
- Outputs are compact typed objects. Organisation API keys, tax identifiers,
  registration numbers, bank account numbers, raw provider envelopes, OAuth
  tokens, and provider error bodies are excluded.
- Xero webhooks are out of scope. A delivery can batch events and the webhook
  is app-global across connected organisations; the current singular Firna
  ingress ABI cannot safely fan out and route that payload. Event support needs
  its own protocol and plan.
- All writes, attachments, payroll, projects, files, bank feeds, journals,
  credit notes, quotes, purchase orders, tax filings, and custom connections
  are out of scope. A later write plan must introduce optional write scopes,
  approval UX, and provider idempotency before any ledger mutation ships.

## Provider Facts (verified 2026-08-03)

- Authorization endpoint:
  `https://login.xero.com/identity/connect/authorize`; token endpoint:
  `https://identity.xero.com/connect/token`; scopes are space-separated.
- Accounting requests use `https://api.xero.com/api.xro/2.0`, a Bearer access
  token, and the selected organisation's `xero-tenant-id` header.
- Xero's token response does not identify an organisation. After exchange, the
  host must call `GET https://api.xero.com/connections` and bind one returned
  `tenantType = ORGANISATION` record by `tenantId`; its `id` is the connection
  id and must not be confused with the tenant id.
- Access tokens expire after 30 minutes. Refresh tokens rotate on every use and
  expire after 60 days of inactivity; an old refresh token has a documented
  30-minute retry grace period after an ambiguous refresh.
- Current limits are five concurrent calls, 60 calls per minute per tenant,
  10,000 calls per minute per app, and 1,000 calls per day per tenant on the
  starter tier (5,000 on higher tiers). New apps also start with a small
  connection allowance, so catalog launch depends on an appropriate Xero app
  tier or certification.
- Rate-limit responses use HTTP 429 plus `Retry-After` and Xero limit headers.
  Collection endpoints support explicit pagination, and Xero recommends
  `If-Modified-Since`, efficient typed filters, and targeted detail requests.

Official references:

- [Accounting API overview](https://developer.xero.com/documentation/api/accounting/overview)
- [Official Accounting OpenAPI](https://github.com/XeroAPI/Xero-OpenAPI/blob/master/xero_accounting.yaml)
- [Authorization-code flow](https://developer.xero.com/documentation/guides/oauth2/auth-flow/)
- [Token types and rotation](https://developer.xero.com/documentation/guides/oauth2/token-types)
- [Tenants and connections](https://developer.xero.com/documentation/guides/oauth2/tenants)
- [Granular scopes](https://developer.xero.com/documentation/guides/oauth2/scopes/)
- [OAuth API limits](https://developer.xero.com/documentation/guides/oauth2/limits/)
- [Efficient paging](https://developer.xero.com/documentation/best-practices/api-call-efficiencies/paging)
- [Accounting response codes](https://developer.xero.com/documentation/api/accounting/responsecodes)

## Platform Dependency (blocking)

The pinned Firna platform revision in `platform.toml`
(`825dffab745c402db8c38501d73d05548a4f238d`) cannot support a safe durable
Xero installation:

1. `standard_oauth2` can map `access_token` and `refresh_token` strings but has
   no expiry metadata, refresh execution, rotation, inactivity keepalive, or
   concurrent-refresh serialization.
2. OAuth completion can select scalar fields only from the token response.
   Xero organisations exist only in the separate `/connections` array, so the
   current host cannot discover or ask the admin to choose an organisation.
3. Tool invocation currently does not carry the installation's
   `provider_account_id` into the host scope, and there is no trusted
   provider-account header injection. Letting the model or component supply
   `xero-tenant-id` would permit accidental cross-organisation access.

The app must not ship with a short-lived token, a hardcoded tenant, a
model-supplied tenant id, or a reconnect-every-30-minutes workaround.
Milestones 2-5 land and adopt the generic platform contract before package
implementation begins.

## Tool Contract (V1)

| Tool | Provider endpoint | Bounded inputs | Compact output |
| --- | --- | --- | --- |
| `xero_get_organisation` | `GET /Organisation` | none | identity, locale, currency, year end, tax basis, status, timezone, lock dates |
| `xero_list_accounts` | `GET /Accounts` | status?, account type?, modified after? | account id, code, name, type, class, status, currency, tax type |
| `xero_list_contacts` | `GET /Contacts` | page, page size, search?, include archived?, modified after? | summary contacts and provider pagination |
| `xero_get_contact` | `GET /Contacts/{ContactID}` | contact UUID | contact profile, roles, defaults, balances, payment terms |
| `xero_list_invoices` | `GET /Invoices` | sales/bill?, statuses?, contact UUID?, search?, page, page size, modified after? | invoice/bill summaries and provider pagination |
| `xero_get_invoice` | `GET /Invoices/{InvoiceID}` | invoice UUID | header, contact, line items, totals, amounts due/paid, status |
| `xero_list_bank_transactions` | `GET /BankTransactions` | receive/spend?, status?, page, page size, modified after? | transaction summaries and provider pagination |
| `xero_get_bank_transaction` | `GET /BankTransactions/{BankTransactionID}` | transaction UUID | account reference, contact, lines, totals, status |
| `xero_list_payments` | `GET /Payments` | status?, page, page size, modified after? | payment id/type/date/amount/status and linked document/account refs |
| `xero_profit_and_loss` | `GET /Reports/ProfitAndLoss` | from/to dates, periods?, timeframe?, cash basis? | report titles and normalized sections/rows/cells |
| `xero_balance_sheet` | `GET /Reports/BalanceSheet` | as-at date, periods?, timeframe?, cash basis? | report titles and normalized sections/rows/cells |
| `xero_trial_balance` | `GET /Reports/TrialBalance` | as-at date, cash basis? | report titles and normalized debit/credit rows |

Shared contract rules:

- UUIDs use canonical UUID strings; dates use `YYYY-MM-DD`; modified-after
  values use RFC 3339 UTC. Page numbers are positive and page size is `1..=100`.
- Report dates are required rather than relying on provider/current-date
  defaults. Profit-and-loss and balance-sheet requests force Xero's standard
  layout so custom tenant layouts do not destabilize the output contract.
- List tools request summary fields where Xero supports them. Detail tools are
  the only path that returns invoice lines, bank-transaction lines, full
  contact defaults, or balances.
- Provider pagination fields (`page`, `pageSize`, `pageCount`, `itemCount`) are
  normalized when present. A caller requests another page explicitly.
- Tool and raw provider responses are capped at 1 MiB, component execution at
  30 seconds, and truncated provider bodies fail as
  `provider_response_too_large`; partial financial records are never returned.
- Stable errors are `invalid_request`, `auth_required`,
  `organisation_disconnected`, `provider_access_denied`, `not_found`,
  `rate_limited` (with safe retry/limit metadata),
  `provider_response_too_large`, `provider_unavailable`, and
  `provider_contract_error`.

## Milestones

### Milestone 1: Complete the Xero Protocol

Land the implementable contract in the platform repository before code. This
is documentation-only work in a separate Firna workspace.

- [ ] Add `docs/protocol/xero-app.md` and
      `docs/protocol/xero-app-tools.md`, each kept near 250 lines, covering the
      product model, exact manifest, OAuth refresh and account selection,
      tenant-header binding, every JSON Schema, request mapping, typed output,
      pagination, redaction, limits, and error mapping.
- [ ] Pin every enum, query parameter, response field, granular scope, and
      report bound against the current Xero docs and official OpenAPI; resolve
      any lag between the OpenAPI's broad scopes and the current granular scope
      documentation in favour of the Developer Centre assignment.
- [ ] Specify local credential lifecycle exactly: disable blocks all use but
      preserves encrypted credentials; uninstall deletes Firna's local access
      and refresh tokens. Neither action revokes a Xero refresh token when that
      could disconnect the same user from other organisations/workspaces;
      provider-wide revocation remains an explicit Xero action.
- [ ] Record read-only V1 and the webhook/write exclusions as normative, not
      implementation TODOs.
- [ ] Validate Markdown and links, commit, push, and run the platform's
      post-push review without auto-fixing findings.

### Milestone 2: Platform OAuth and Tenant Backend

Implement provider-neutral backend support in `futex-ai/firna`. No UI or
mockup work belongs in this milestone.

- [ ] Extend `standard_oauth2` with typed access-token expiry, atomic rotating
      refresh tokens, an inactivity deadline/keepalive schedule, and stable
      refresh failure states. Serialize refresh per installation and retry a
      safe GET at most once after successful refresh.
- [ ] Preserve Xero's old refresh token until the replacement pair and expiry
      metadata commit atomically; allow the documented grace retry after an
      ambiguous exchange and turn terminal `invalid_grant` into `auth_required`.
- [ ] Add constrained post-token account discovery metadata: fixed HTTPS GET,
      Bearer auth, bounded response, array selector, id/name/type selectors,
      allowed account types, and opaque pending-selection state.
- [ ] Add a secure account-selection command that validates the option against
      the pending discovery result, stores `tenantId` as provider account,
      stores only the non-secret connection id and display name as account
      metadata, and then activates the installation.
- [ ] Pass the bound provider account into tool host scope and add manifest-
      reviewed header injection for `xero-tenant-id`. Reject component-supplied
      conflicts and calls without the bound installation/account.
- [ ] Put clocks, refresh scheduling, OAuth HTTP, account discovery, vault,
      store, and transaction behavior behind traits; use `unimock` plus store,
      migration, concurrency, cross-tenant, redaction, and failure-injection
      tests.
- [ ] Update platform protocol docs and crate READMEs, run the full platform
      checks, commit, push, and review.

### Milestone 3: Xero Connection Mockups

Tags: mockup

Specify the new Settings continuation before UI implementation.

- [ ] Update the existing Installed apps mockup hierarchy with mobile and
      desktop variants for choosing a real Xero organisation, an empty/error
      discovery state, active organisation detail, and reconnect-required
      state. Use no sample financial metrics or implementation terminology.
- [ ] Add or update a user flow that reuses the owning standalone screens and
      links back to each source screen.
- [ ] Run `npm run mockups:build`, `mockups:check`, `mockups:test`, and
      `mockups:typecheck`; open every changed generated page directly from disk
      for visual smoke testing.
- [ ] Commit, push, and run the platform post-push review.

### Milestone 4: Xero Connection UI

Tags: ui

Implement the reviewed account-selection states in the universal app; no
backend work belongs here.

- [ ] Render the new account-selection next step with real discovery data,
      explicit selection/confirmation, accessible loading/empty/error states,
      and plain product copy such as “Choose the Xero organisation to connect.”
- [ ] Show the bound organisation name on app detail and reconnect screens
      without exposing tenant ids, connection ids, scopes, or token state.
- [ ] Cover web/mobile callback continuation, stale selections, cancellation,
      reconnect, and session restoration with component and smoke tests.
- [ ] Run all relevant app format, lint, type, test, build, and smoke checks;
      commit, push, and review in the platform repository.

### Milestone 5: Adopt the Supporting Platform Revision

- [ ] After the platform work merges, update `platform.toml`, the deployment
      workflow pin, the four existing runtime-test manifests, their lockfiles,
      and the root CLI install revision together.
- [ ] Add compatibility/audit coverage that rejects a partial pin update.
- [ ] Run `cargo xtask check` before beginning the Xero package and keep all
      existing apps green.

### Milestone 6: Provision the Xero Developer App

- [ ] Create a production confidential Web App named `Firna` in the Xero
      Developer Centre with the exact callback
      `https://firna.ai/oauth/xero/callback`; do not use Custom Connections.
- [ ] Request only the V1 granular scopes, confirm the current connection and
      daily-call tier is sufficient for the intended rollout, and block public
      catalog launch until any required partner/certification step is approved.
- [ ] Put the public client id in the manifest and transfer the client secret
      directly to Google Secret Manager as
      `firna-prod-app-xero-client-secret`. Never place the secret in Git,
      `.context`, shell history, logs, screenshots, or chat.
- [ ] Record only redacted app id, callback, tier, collaborators, and secret
      version in the implementation handoff.

### Milestone 7: Build the Xero Package

- [ ] Scaffold `apps/xero` as a standalone component and runtime-test
      workspace with committed lockfiles, required READMEs, and an official
      Xero brand asset (SVG source, rendered PNG/base64, accessible color pair).
- [ ] Write manifest `1.0.0`: `source.kind: community`, explicit install,
      `api.xero.com` only, public client id, required `client_secret`, the
      reviewed OAuth/discovery/tenant-header contract, all 12 tools, no ingress
      or events, 1 MiB response limits, and 30-second component limits.
- [ ] Implement typed `input`, `output`, `tools`, `host`, `error`, and dispatch
      modules under 300 lines. Keep host HTTP behavior behind a trait and use
      opaque installation credentials; the component never receives tokens or
      chooses a tenant id.
- [ ] Normalize Xero dates, pagination, entities, report rows, rate-limit
      headers, and errors into the V1 contract while dropping unknown or
      sensitive provider fields.
- [ ] Add `_tests_` unit modules with `unimock` coverage for every schema,
      request, filter, response mapper, truncation path, permission/rate/auth
      error, and assurance that no write method or unreviewed endpoint can be
      dispatched.
- [ ] Update package/component/runtime-test READMEs and link the platform Xero
      protocols; build, lint, and test the component for host and
      `wasm32-unknown-unknown`.

### Milestone 8: Runtime and Repository Integration

- [ ] Run every tool through the real Wasm component with `WasmHostMock`,
      asserting exact method/path/query, Bearer injection reference, trusted
      tenant-header injection, one-request behavior, normalized output, and
      redaction. No automated test uses live Xero credentials.
- [ ] Add package tests for manifest identity, scopes, OAuth refresh/discovery,
      account binding, tools, schemas, side effects, host allowlist, limits,
      assets, and docs.
- [ ] Add Xero's component and runtime manifests to `xtask/src/check.rs` and
      strengthen the inventory test to discover package manifests, preventing
      another app from being silently omitted from the check plan.
- [ ] Update root and `apps/README.md` catalog summaries and local commands;
      confirm deployment discovery needs no special-case workflow code beyond
      the one new Secret Manager value.
- [ ] Run `firna apps validate apps/xero`, `firna apps package apps/xero`, all
      targeted format/clippy/build/unit/runtime tests, and the repository audit.

### Milestone 9: Authorized Live Smoke Test

- [ ] Install the reviewed package in a nominated non-production Firna
      workspace and connect a Xero Demo Company, not a real operating ledger.
- [ ] Prove account discovery and explicit selection with one and multiple
      authorised organisations; verify the installation cannot access an
      unselected tenant even when the access token can see it.
- [ ] Exercise access-token expiry, atomic refresh-token rotation, concurrent
      calls during refresh, and the inactivity keepalive without exposing
      credentials.
- [ ] Smoke all 12 tools against real Demo Company data, including a second
      explicit collection page, a modified-after query, report permission
      handling, normalized pagination/rate metadata, and zero provider writes.
- [ ] Record only redacted outcomes and current limit usage in the PR.

### Milestone 10: Complete Repository Gate

- [ ] Run `cargo fmt --all -- --check`, every standalone component/runtime
      format and clippy command, Rust file-length lint/audit, and all relevant
      tests with a 100% pass rate.
- [ ] Run `cargo xtask check` and resolve every compile, lint, package,
      workflow, documentation, or test failure before proceeding.
- [ ] Inspect `git diff --check`, the complete diff, name-status, and deletion
      diff against `origin/main`; stop for any unapproved mainline removal.

### Milestone 11: Commit and Push Checked Work

- [ ] Fetch `origin/main`, capture the source tip, audit additions since the
      merge base, and resolve overlaps path by path while preserving mainline
      features.
- [ ] Run `git add -A`, commit every package file, asset, lockfile, test, and
      document using a Conventional Commit title at most 50 characters (for
      example `feat(xero): add read-only accounting tools`), and push the
      current branch without renaming it.
- [ ] Recheck the committed name-status and deletion diff against
      `origin/main`.

### Milestone 12: Post-Push Review

- [ ] After the push, run `cargo xtask review` against `origin/main`.
- [ ] Do not automatically fix findings. Report each as a numbered item with
      severity, feature/codebase context, impact of doing nothing, lettered
      solution options, and a recommended option.
- [ ] When every preceding TODO is complete and review is reported, move this
      plan from Active to Completed in `plans/README.md`.

## Completion Criteria

- A workspace admin can connect and explicitly select one Xero organisation;
  Firna refreshes credentials durably and host-enforces that tenant boundary.
- All 12 tools return only live Xero data through bounded, typed, read-only
  requests, with no model-visible credentials, tenant selector, raw financial
  envelope, or mutation path.
- Unit, Wasm runtime, platform OAuth/tenant, cross-tenant, UI, package, live
  Demo Company, and full repository checks pass.
- The checked branch is committed and pushed, the post-push review is reported
  without silent fixes, and Xero's connection tier supports the approved
  rollout before catalog release.
