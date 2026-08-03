# Xero Accounting and VAT App

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-03

## Outcome

Add `apps/xero`, an explicitly installed first-party app that lets a Firna
workspace connect one Xero organisation, read its core accounting data, and
perform controlled writes. The first release covers contacts, chart-of-account
records, sales invoices, bills, credit notes, payments, spend/receive-money
bank transactions, bank transfers, manual journals, and financial reports.

Every external write is prepared as an immutable proposal and requires an
authorised human to approve the exact payload before Firna sends it. Firna
enforces tenant binding, stale-record checks, idempotency, an audit trail, and
provider reconciliation; an agent cannot approve its own proposal or turn a
read tool into a write.

For UK organisations, the app also supports preparing and filing a VAT return.
Xero's public Accounting API does not expose UK VAT filing, so filing uses a
second, optional HMRC Making Tax Digital (MTD) connection and the official VAT
API. Xero remains the digitally linked bookkeeping source. A VAT return is
submitted only from a reviewed, immutable nine-box draft after the authorised
person accepts HMRC's final declaration. Creating the provider applications,
passing Xero/HMRC production approval, transferring secrets, and authorising a
real filing are external actions requiring human operators.

## Product Contract

### Organisation and credentials

- One active Xero organisation per Firna workspace. OAuth may expose several
  authorised Xero connections, but an admin explicitly selects one and that
  `tenantId` becomes the installation's immutable Xero account binding.
- The Xero tenant id is never a model-visible tool input. Trusted host code
  injects `xero-tenant-id`; the component cannot override it or address an
  arbitrary organisation.
- The optional HMRC VAT connection is a separate workspace-owned OAuth grant.
  An admin enters the nine-digit VAT registration number (VRN), Firna verifies
  it through an authorised HMRC read, and the verified VRN becomes the HMRC
  account binding. VAT tools cannot accept an arbitrary VRN.
- A VAT filing requires both bindings to identify the same UK business.
  Installation normalizes Xero's country and VAT number, verifies that exact
  VRN through the authorised HMRC customer call, and blocks mismatches without
  revealing identifiers to the model.
- Install policy is `explicit`. The Xero authorising user needs the provider
  roles required by each endpoint. HMRC business users or agents must grant the
  correct MTD authority for the bound VRN.
- Xero uses a confidential Web App and its authorization-code flow. HMRC uses
  a separate confidential server-side authorization-code flow. Access and
  rotating refresh tokens stay behind Firna's opaque credential boundary.

### Scopes

- Xero requests `offline_access` and the minimum granular scopes needed for
  this release: `accounting.settings`, `accounting.contacts`,
  `accounting.invoices`, `accounting.payments`,
  `accounting.banktransactions`, `accounting.manualjournals`,
  `accounting.reports.profitandloss.read`,
  `accounting.reports.balancesheet.read`, and
  `accounting.reports.trialbalance.read`.
- The write-capable Xero scopes replace their corresponding `.read` scopes;
  the app manifest and install screen identify which record families can be
  changed. No deprecated broad accounting scope is requested.
- HMRC VAT tools request `read:vat`; filing additionally requires `write:vat`.
  The HMRC grant is optional until a workspace enables UK VAT filing.

### Write and approval policy

- Every Xero or HMRC mutation is `external_write` and requires a fresh human
  approval. No accounting or tax write can be auto-approved by amount,
  workspace policy, agent role, repeated use, schedule, or prior approval.
- The platform first invokes a read-only preflight export. It resolves provider
  names and current state, validates tax/account combinations and lock dates,
  and returns a canonical proposal. The host stores its hash, Xero tenant,
  provider record versions, operation, amount/currency, and expiry.
- Approval UI shows the exact business, operation, affected records, line
  items, tax treatment, totals, and irreversible consequences. Approval is
  bound to the proposal hash and expires after 15 minutes. Editing any field or
  observing a changed provider record invalidates it and requires a new review.
- The host, not the model or component, supplies the single-use approval
  receipt and Xero `Idempotency-Key`. The commit export rejects absent, expired,
  reused, wrong-user, wrong-workspace, wrong-tenant, or wrong-payload receipts.
- Creation tools make drafts where Xero supports them. Approving/authorising an
  invoice or credit note, posting a journal, creating a bank transaction,
  moving money between bank accounts, recording/reversing a payment, deleting
  or voiding a record, and filing VAT are separate approvals.
- Mutations are single-record operations. Bulk writes, raw provider payloads,
  arbitrary endpoint/method inputs, and model-selected tenant or VRN values are
  not exposed.
- Ambiguous timeouts are reconciled with an exact provider read before any
  retry. Firna never generates a new idempotency key simply because the first
  response was lost.
- Every proposal, decision, provider request fingerprint, provider result,
  reconciliation, and actor is retained in an immutable workspace audit trail.
  Tokens, bank account numbers, full tax identifiers, and provider error bodies
  are excluded.

### UK VAT filing

- VAT filing is implemented against HMRC's VAT (MTD) API, not an undocumented
  Xero endpoint or browser automation. The required provider calls are
  `GET /organisations/vat/{vrn}/obligations` and
  `POST /organisations/vat/{vrn}/returns`.
- V1 supports UK organisations using standard accrual or cash VAT accounting
  only after the calculation protocol and accountant-reviewed conformance
  fixtures pass. Flat-rate, partial-exemption, group, retail, margin, annual,
  insolvency, and other specialist schemes render an explicit unsupported state
  and cannot be filed.
- `xero_vat_prepare_return` builds the nine boxes from a complete, digitally
  linked Xero ledger snapshot. It records every included source record, tax
  rule, rounding decision, late item, and structured adjustment. The model
  cannot supply final box values directly.
- An authorised user can add a structured VAT adjustment with amount, box,
  reason, and evidence reference. The draft is then recalculated and requires
  a new review. Silent balancing entries and free-form replacement totals are
  forbidden.
- `xero_vat_submit_return` accepts only an immutable `draft_id`. Before filing,
  Firna rechecks the HMRC obligation is open, the period has ended, both OAuth
  grants are valid, the Xero source snapshot has not changed, all boxes satisfy
  HMRC arithmetic/rounding constraints, and the approving user has tax-filing
  permission.
- The approval surface displays the organisation, VRN suffix, obligation dates,
  all nine boxes, adjustments, source snapshot time, and HMRC declaration. The
  human must actively confirm the declaration; an agent cannot set
  `finalised: true` or file on a schedule.
- A successful response stores HMRC's receipt identifiers and timestamps,
  fetches the filed return for comparison, and makes the receipt available to
  the user. Duplicate or ambiguous submissions are reconciled before another
  attempt. VAT payment initiation is not part of this app.

### Explicit exclusions

- Xero bank transactions (`BankTransactions`) and bank transfers are in scope.
  Importing bank statement lines, creating a bank feed, and reconciling a bank
  statement are not: the public Accounting API does not support those actions,
  and the Bank Feeds API is restricted to certified financial institutions.
- Payroll, projects, files, attachments, inventory, quotes, purchase orders,
  repeating invoices, batch payments, expense claims, bank feeds, and payment
  initiation are out of scope for this release.
- Contact bank-account details are neither returned nor writable. Account
  creation/update is limited to non-bank chart accounts; existing bank accounts
  are read-only references for transactions, transfers, and payments. The app
  does not change Xero tax rates, tracking categories, organisation settings,
  user permissions, or bank-account type.
- Xero webhooks remain out of scope. A delivery can batch events and the webhook
  is app-global across connected organisations; event routing needs its own
  protocol and plan.

## Provider Facts (verified 2026-08-03)

### Xero

- Authorization endpoint:
  `https://login.xero.com/identity/connect/authorize`; token endpoint:
  `https://identity.xero.com/connect/token`; scopes are space-separated.
- Accounting requests use `https://api.xero.com/api.xro/2.0`, a Bearer access
  token, and the selected organisation's `xero-tenant-id` header.
- Xero's token response does not identify an organisation. After exchange, the
  host must call `GET https://api.xero.com/connections` and bind a returned
  `tenantType = ORGANISATION` record by `tenantId`; its `id` is the connection
  id and must not be confused with the tenant id.
- Access tokens expire after 30 minutes. Refresh tokens rotate on every use and
  expire after 60 days of inactivity; an old refresh token has a documented
  30-minute retry grace period after an ambiguous refresh.
- Xero supports idempotency for mutating `POST`, `PUT`, and `PATCH` calls using
  `Idempotency-Key`, but caches keys for only six minutes. Firna therefore also
  needs its own durable mutation ledger and read-after-timeout reconciliation.
- Spend/receive-money bank transactions have no draft state and become
  `AUTHORISED` when created. Bank transfers and payments are created or reversed
  rather than freely edited. These operations always use the strongest approval
  treatment.
- Apps writing tax-bearing transactions must use tax rates retrieved from the
  connected Xero organisation. Xero asks integrations not to send calculated
  tax amounts unless agreed with its developer team; V1 sends `TaxType` and lets
  Xero calculate tax, then verifies returned totals.
- Current limits are five concurrent calls, 60 calls per minute per tenant,
  10,000 calls per minute per app, and 1,000 calls per day per tenant on the
  starter tier (5,000 on higher tiers). New apps have a limited connection tier,
  so catalog launch depends on the appropriate Xero tier or certification.

### HMRC VAT (MTD)

- HMRC's production API base is `https://api.service.hmrc.gov.uk`; OAuth for
  user-restricted endpoints uses the authorization-code grant. Access tokens
  last four hours, refresh tokens are single-use, and a grant must be renewed
  after 18 months.
- The VAT API supports reading obligations and filed returns and submitting a
  return to `POST /organisations/vat/{vrn}/returns` with `write:vat`.
  `finalised: true` is the user's legal declaration, not a draft flag.
- HMRC requires legally mandated fraud-prevention headers for every VAT API
  call. Production approval requires evidence from the Test Fraud Prevention
  Headers API, sandbox testing, terms/privacy/support readiness, and HMRC review.
- The public Xero Accounting API has no UK VAT-return preparation or filing
  endpoint. Firna must not claim that an HMRC filing was submitted “through
  Xero,” and must not automate Xero's browser UI.

Official references:

- [Xero Accounting API overview](https://developer.xero.com/documentation/api/accounting/overview)
- [Official Xero Accounting OpenAPI](https://github.com/XeroAPI/Xero-OpenAPI/blob/master/xero_accounting.yaml)
- [Xero OAuth scopes](https://developer.xero.com/documentation/guides/oauth2/scopes/)
- [Xero token rotation](https://developer.xero.com/documentation/guides/oauth2/token-types)
- [Xero tenants and connections](https://developer.xero.com/documentation/guides/oauth2/tenants)
- [Xero idempotent requests](https://developer.xero.com/documentation/guides/idempotent-requests/idempotency/)
- [Xero bank transactions](https://developer.xero.com/documentation/api/accounting/banktransactions)
- [Xero bank transfers](https://developer.xero.com/documentation/api/accounting/banktransfers)
- [Xero payments](https://developer.xero.com/documentation/api/accounting/payments)
- [Xero tax integrity guidance](https://developer.xero.com/documentation/best-practices/data-integrity/taxes)
- [Xero bank-statement limitations](https://developer.xero.com/documentation/api/accounting/bankstatements)
- [Xero Bank Feeds access](https://developer.xero.com/documentation/api/bankfeeds/overview)
- [HMRC VAT (MTD) API](https://developer.service.hmrc.gov.uk/api-documentation/docs/api/service/vat-api/1.0)
- [HMRC VAT end-to-end guide](https://developer.service.hmrc.gov.uk/guides/vat-mtd-end-to-end-service-guide/)
- [HMRC OAuth for user endpoints](https://developer.service.hmrc.gov.uk/api-documentation/docs/authorisation/user-restricted-endpoints)
- [HMRC fraud-prevention headers](https://developer.service.hmrc.gov.uk/guides/fraud-prevention/)

## Platform Dependencies (blocking)

The pinned Firna platform revision in `platform.toml`
(`825dffab745c402db8c38501d73d05548a4f238d`) cannot safely provide this
contract:

1. `standard_oauth2` maps token strings but has no expiry metadata, refresh
   execution, rotating-token transaction, inactivity keepalive, or
   concurrent-refresh serialization for Xero or HMRC.
2. OAuth completion cannot run Xero's separate `/connections` discovery and
   select an organisation. Tool invocation does not expose a trusted,
   installation-bound provider account for host-injected tenant headers.
3. App tools can name multiple auth requirements, but invocation does not carry
   separate provider-account contexts for Xero and HMRC or support a verified
   VRN path binding.
4. `external_write` is descriptive metadata, not an execution gate. The current
   approval request is not cryptographically bound to a tool, payload, tenant,
   provider versions, or one-time commit and may auto-approve small amounts.
5. There is no durable app-mutation proposal/idempotency ledger, preflight and
   commit ABI, optimistic provider-state check, ambiguous-result
   reconciliation, or immutable financial audit record.
6. HMRC fraud-prevention headers require trusted request, user, device, public
   IP, and server data that a Wasm component must not fabricate or receive as
   arbitrary model input.
7. VAT preparation can require many bounded provider pages and durable snapshot
   state; it cannot be forced through one 30-second component call without a
   resumable job contract.

The app must not ship with direct model-authorised writes, a `confirm: true`
input, a hardcoded tenant/VRN, a reconnect-every-expiry workaround, browser
automation, or a VAT calculator whose source coverage is incomplete.
Milestones 3-8 land and adopt the generic platform contracts before the package
can expose write or VAT tools.

## Tool Contract

### Read and discovery tools

| Tools | Provider operation | Contract |
| --- | --- | --- |
| `xero_get_organisation` | `GET /Organisation` | identity, locale, currency, tax basis, status, timezone, and lock dates; identifiers redacted |
| `xero_list_accounts`, `xero_list_tax_rates`, `xero_list_tracking_categories` | corresponding Accounting GETs | active configuration needed to prepare valid writes; typed filters only |
| `xero_find_contacts` | `GET /Contacts[/{id}]` | list/search or one detail record; bank and tax identifiers excluded |
| `xero_find_invoices` | `GET /Invoices[/{id}]` | sales invoices and bills with bounded list/detail modes |
| `xero_find_credit_notes` | `GET /CreditNotes[/{id}]` | credit-note state, lines, totals, and allocations |
| `xero_find_bank_transactions` | `GET /BankTransactions[/{id}]` | spend/receive-money records and reconciliation status |
| `xero_find_bank_transfers` | `GET /BankTransfers[/{id}]` | transfers between bound-tenant bank accounts |
| `xero_find_payments` | `GET /Payments[/{id}]` | payment, linked document, account reference, and reversal state |
| `xero_find_manual_journals` | `GET /ManualJournals[/{id}]` | journal lines and draft/posted/voided state |
| `xero_profit_and_loss`, `xero_balance_sheet`, `xero_trial_balance` | corresponding report GETs | required dates and normalized report rows |
| `xero_vat_get_customer` | HMRC `GET /organisations/vat/{vrn}/information` | verified VAT status and supported-scheme signals |
| `xero_vat_list_obligations` | HMRC obligations GET | bounded open/fulfilled periods from HMRC |
| `xero_vat_get_return` | HMRC filed-return GET | filed nine boxes for a bound period key |
| `xero_vat_list_liabilities`, `xero_vat_list_payments` | HMRC liability/payment GETs | bounded date ranges and normalized amounts |
| `xero_vat_prepare_return` | resumable Xero snapshot and local computation | immutable draft, provenance summary, adjustments, warnings, and hash; no provider mutation |

### Write tools

| Tools | Provider operation | Mutation boundary |
| --- | --- | --- |
| `xero_save_contact` | create/update `Contacts` | one contact; excludes bank details and merge/archive operations |
| `xero_save_account`, `xero_archive_account` | create/update/archive `Accounts` | one non-system, non-bank chart account; no delete or type change |
| `xero_save_invoice_draft` | create/update `Invoices` | one `DRAFT` sales invoice or bill; provider-derived tax totals |
| `xero_transition_invoice` | update one invoice status | typed `submit`, `approve`, `void`, or `delete` transition valid from current state |
| `xero_save_credit_note_draft` | create/update `CreditNotes` | one draft sales/purchase credit note |
| `xero_transition_credit_note` | update one credit-note status | typed approve, void, or delete transition |
| `xero_allocate_credit_note` | create credit-note allocation | one amount against one authorised invoice |
| `xero_save_bank_transaction` | create/update `BankTransactions` | one `SPEND` or `RECEIVE`; creation is immediately authorised |
| `xero_delete_bank_transaction` | mark one bank transaction deleted | only when Xero permits; no statement-line reconciliation |
| `xero_create_bank_transfer` | create `BankTransfers` | one transfer between two distinct active bank accounts |
| `xero_delete_bank_transfer` | mark one transfer deleted | provider-supported reversal semantics shown in approval |
| `xero_create_payment` | create `Payments` | one payment against one authorised invoice/credit/prepayment record |
| `xero_reverse_payment` | mark one payment deleted | reversal only; original record remains auditable |
| `xero_save_manual_journal_draft` | create/update `ManualJournals` | balanced draft with typed tax and tracking references |
| `xero_transition_manual_journal` | update journal status | typed post, void, or delete transition valid from current state |
| `xero_vat_submit_return` | HMRC VAT return POST | immutable prepared draft only; always human-approved and final |

### Shared schema and error rules

- UUIDs are canonical; dates use `YYYY-MM-DD`; timestamps use RFC 3339 UTC;
  currency uses ISO 4217; money is a decimal string with provider-supported
  precision and never a binary floating-point JSON number.
- List tools expose one bounded page and never drain later pages automatically.
  Raw Xero `where` and `order` expressions are not model-visible.
- Save inputs are closed typed objects. Updates require the provider id and
  `expected_updated_at`; transitions additionally require `expected_status`.
- Line items are bounded, descriptions and references have explicit lengths,
  account/tax/tracking ids must come from the bound tenant, and debits/credits
  or transfer accounts must balance before a proposal can be shown.
- Tool and raw provider responses are capped at 1 MiB. VAT snapshot pages are
  individually capped and the resumable job has explicit page, record, time,
  and retry budgets. Partial records or partial VAT returns are never emitted.
- Stable errors include `invalid_request`, `approval_required`,
  `approval_expired`, `approval_mismatch`, `stale_record`, `auth_required`,
  `organisation_disconnected`, `vat_connection_required`,
  `business_identity_mismatch`, `unsupported_vat_scheme`, `period_locked`,
  `provider_access_denied`, `provider_validation_failed`, `not_found`,
  `rate_limited`, `ambiguous_provider_result`, `provider_response_too_large`,
  `provider_unavailable`, and `provider_contract_error`.

## Milestones

### Milestone 1: Complete the Xero and VAT Protocols

Create the implementable contracts in the platform repository before code.

- [ ] Add concise `docs/protocol/xero-app.md`,
      `docs/protocol/xero-app-read-tools.md`,
      `docs/protocol/xero-app-write-tools.md`, and
      `docs/protocol/xero-vat-filing.md`, splitting further if any protocol
      would exceed roughly 250 lines.
- [ ] Pin every JSON Schema, enum, field limit, provider method/path, Xero
      status transition, permission, scope, response, error, redaction,
      preflight, approval, idempotency, reconciliation, and audit rule against
      the current official documentation and OpenAPI files.
- [ ] Specify Xero/HMRC credential disable, reconnect, expiry, revocation, and
      uninstall behavior without disconnecting unrelated provider grants.
- [ ] Record the exact supported VAT schemes and every explicit exclusion as
      normative behavior, not implementation TODOs.
- [ ] Validate Markdown and links, commit, push, and run the platform's
      post-push review without automatically fixing findings.

### Milestone 2: Validate the UK VAT Calculation Contract

Finish the tax-domain design before backend or user-interface implementation.

- [ ] Engage a qualified UK VAT adviser to review source-record inclusion,
      cash/accrual timing, credit notes, reverse charges, imports/acquisitions,
      late claims, rounding, adjustments, locked periods, and all nine boxes.
- [ ] Build versioned, anonymised golden fixtures for supported schemes with
      expected boxes and source-to-box provenance; include amended, late,
      duplicate, refund, zero, and unsupported-scheme cases.
- [ ] Prove the public Xero endpoints expose every source fact required for the
      supported contract. Narrow the supported schemes or block filing if any
      value would require inference from unavailable Xero state.
- [ ] Document record retention, export, privacy, support, incident response,
      correction-after-filing, and evidence requirements for HMRC production
      review.

### Milestone 3: Platform OAuth and Provider-Account Backend

Implement provider-neutral credential support in `futex-ai/firna`. No UI or
mockup work belongs in this milestone.

- [ ] Extend `standard_oauth2` with typed expiry, atomic single-use refresh-token
      rotation, Xero inactivity keepalive, HMRC 18-month reauthorisation, and
      stable terminal refresh states. Serialize refresh per grant.
- [ ] Add constrained post-token account discovery for Xero and verified
      operator-entered account binding for providers such as HMRC.
- [ ] Carry a provider account per auth requirement into tool scope; support
      trusted header and path bindings while rejecting component/model
      conflicts.
- [ ] Support one app installation with independently connectable Xero and HMRC
      workspace OAuth requirements and clear partial-connection states.
- [ ] Put clocks, OAuth HTTP, discovery, verification, vault, store, and
      transaction behavior behind traits; cover concurrency, cross-tenant,
      redaction, refresh, and failure injection with `unimock` and integration
      tests.
- [ ] Update platform protocols and crate READMEs, run full platform checks,
      commit, push, and review.

### Milestone 4: Platform Financial Write Controls

Add the reusable execution boundary required by accounting mutations.

- [ ] Add manifest-declared approval policy for `external_write`, including a
      `human_required` mode that can never use amount-based auto-approval.
- [ ] Add preflight/commit component exports and a durable proposal record bound
      to canonical payload hash, tool/version, workspace, providers, current
      record versions, proposer, expiry, and one authorised resolver.
- [ ] Inject a single-use approval receipt and provider idempotency key only
      after approval. Atomically consume the receipt with mutation start and
      reject replay or changed payloads.
- [ ] Add durable mutation states for pending, approved, executing, succeeded,
      rejected, expired, stale, ambiguous, reconciled, and failed; make restarts
      and concurrent workers safe.
- [ ] Add provider read-after-timeout reconciliation and prohibit blind retry
      outside the provider idempotency window.
- [ ] Add immutable redacted audit/event records and finance/tax permissions;
      tax filing and destructive accounting transitions always require an
      authorised human with no policy override.
- [ ] Add service, store, migration, RPC, agent-tool, and adversarial tests for
      tampering, replay, wrong tenant, stale state, cancellation, timeout,
      concurrency, compaction, and privilege changes.

### Milestone 5: Platform HMRC Compliance Transport

Implement trusted HMRC request behavior outside the Wasm component.

- [ ] Add manifest-reviewed HMRC API version/media type handling and server-side
      fraud-prevention header generation from trusted request, user, device,
      network, and server context.
- [ ] Ensure the component and model cannot view, set, or override protected
      fraud, authorization, VRN-path, host, or forwarding headers.
- [ ] Add the Test Fraud Prevention Headers API workflow, diagnostics that do
      not leak personal data, version monitoring, and auditable compliance
      evidence.
- [ ] Cover browser, mobile, CLI, agent-user, proxy, IPv4/IPv6, missing-context,
      and header-validation cases required by HMRC's current specification.

### Milestone 6: VAT Snapshot and Calculation Backend

Build the typed, versioned tax engine; no UI or mockup work belongs here.

- [ ] Implement a resumable bounded Xero snapshot job that records provider ids,
      update timestamps, hashes, tax types, and inclusion decisions without
      persisting unnecessary contact or bank data.
- [ ] Implement the adviser-reviewed cash/accrual rules as pure typed modules,
      with decimal arithmetic and explicit rounding; do not use raw JSON or
      model-generated calculations.
- [ ] Persist immutable draft versions, source provenance, structured
      adjustments, calculation-version id, warnings, and stale-source state.
- [ ] Recheck the complete source watermark immediately before filing and force
      regeneration when any relevant record changes.
- [ ] Pass every golden, property, boundary, mutation, and unsupported-scheme
      fixture with deterministic results and human-readable provenance.

### Milestone 7: Connection and Write-Approval Mockups

Tags: mockup

Specify the connection and ordinary accounting-write journeys before UI work.

- [ ] Extend the Installed apps mockup hierarchy with mobile and desktop screens
      for Xero organisation selection, optional HMRC connection, verified
      business matching, reconnect-required, and unsupported states.
- [ ] Add nested pages for accounting proposal review, stale proposal, success,
      provider validation failure, and ambiguous-result reconciliation. Keep no
      more than five screens on one generated screen-spec page.
- [ ] Show plain product language, real-shaped empty states, exact totals and
      consequences, and no tenant ids, VRNs, token/scopes, test badges, or
      implementation terminology.
- [ ] Build user flows only from standalone screen components and link every
      flow screen back to its source.
- [ ] Run `npm run mockups:build`, `mockups:check`, `mockups:test`, and
      `mockups:typecheck`; open every changed page directly from disk.

### Milestone 8: VAT Preparation and Filing Mockups

Tags: mockup

Specify the high-stakes VAT journey separately from ordinary writes.

- [ ] Add nested mobile/desktop screens for obligation selection, snapshot
      progress, nine-box draft with provenance, structured adjustment review,
      unsupported scheme, stale ledger, final declaration, HMRC receipt, and
      duplicate/ambiguous filing.
- [ ] Make the final action unmistakably a filing to HMRC, show that it cannot
      be undone in Firna, and require an active declaration control rather than
      a generic confirmation.
- [ ] Have a UK VAT adviser review user-facing terminology, box labels,
      warnings, and declaration placement before implementation.
- [ ] Run all mockup build/check/test/typecheck and direct-file visual smoke
      checks, committing generated HTML with its React source.

### Milestone 9: Connection and Financial Approval UI

Tags: ui

Implement the approved connection and general write-review designs.

- [ ] Render independent Xero/HMRC connection states with real discovery and
      verification data, explicit account confirmation, and accessible
      loading/empty/error/reconnect behavior.
- [ ] Render server-owned immutable proposal details and provider-resolved names;
      the client cannot alter hidden payloads or manufacture approval receipts.
- [ ] Require a fresh authenticated finance-authorised user for writes, support
      reject/cancel/expiry/stale/ambiguous states, and announce outcomes
      accessibly on mobile and web.
- [ ] Cover callback restoration, cross-workspace isolation, revoked roles,
      double clicks, stale browser tabs, and resumed agent turns with component
      and smoke tests.

### Milestone 10: VAT Preparation and Filing UI

Tags: ui

Implement the reviewed VAT-specific experience; no backend work belongs here.

- [ ] Build the obligation, progress, draft, provenance, adjustment, unsupported,
      stale, declaration, receipt, and support-link screens from real backend
      state.
- [ ] Require the authorised filer to inspect all nine boxes and actively accept
      HMRC's declaration; never preselect, auto-accept, or allow an agent to
      satisfy it.
- [ ] Provide an exportable audit/receipt view without exposing OAuth tokens,
      full VRNs, contact details, or raw fraud-prevention data.
- [ ] Run relevant format, lint, type, test, build, accessibility, and mobile/web
      smoke checks; commit, push, and review in the platform repository.

### Milestone 11: Adopt the Supporting Platform Revision

- [ ] After platform work merges, update `platform.toml`, deployment workflow
      pin, all existing runtime-test manifests and lockfiles, and the root CLI
      install revision together.
- [ ] Add compatibility/audit coverage that rejects a partial pin update.
- [ ] Run `cargo xtask check` and keep all existing apps green before beginning
      the Xero package.

### Milestone 12: Provision Xero and HMRC Applications

- [ ] Create the production confidential Xero Web App named `Firna` with the
      exact callback `https://firna.ai/oauth/xero/callback`; request only the
      reviewed granular scopes and confirm rollout tier/certification.
- [ ] Create HMRC sandbox and production applications with callback
      `https://firna.ai/oauth/hmrc/callback`, subscribe to VAT (MTD), complete
      required sandbox and fraud-header tests, accept terms, and obtain
      production approval before enabling filing.
- [ ] Put the public Xero/HMRC client ids in distinct manifest environment
      fields. Transfer the secrets directly to
      `firna-prod-app-xero-client-secret` and
      `firna-prod-app-hmrc-client-secret` in Google Secret Manager. Never place
      secrets in Git, `.context`, shell history, logs, screenshots, fixtures,
      or chat.
- [ ] Complete Xero tax-write review and HMRC privacy, terms, support, security,
      penetration-test, and production questionnaires. Block catalog claims for
      VAT filing until approval is granted.
- [ ] Record only redacted app ids, callbacks, scopes, tiers, collaborators,
      approvals, and secret versions in the implementation handoff.

### Milestone 13: Build the Xero Package

- [ ] Scaffold `apps/xero` as a standalone component and runtime-test workspace
      with committed lockfiles, required READMEs, and an approved Xero brand
      asset.
- [ ] Write manifest `1.0.0` with `source.kind: community`, explicit install,
      Xero and optional HMRC auth requirements, reviewed hosts, all
      read/write/VAT tools, protected account bindings, human-required writes,
      response limits, and no ingress/events.
- [ ] Implement small typed modules for inputs, outputs, preflight, commit,
      provider clients, VAT jobs, errors, and dispatch. Keep all impure behavior
      behind traits and never expose tokens, approval receipts, tenant ids, or
      VRNs to the component/model.
- [ ] Use live Xero tax rates, decimal money, provider state transitions,
      idempotency, expected versions, normalized errors, and read-after-timeout
      reconciliation exactly as specified.
- [ ] Add external `_tests_` modules with `unimock` coverage for every schema,
      request mapper, response mapper, state transition, approval failure,
      stale record, tax validation, timeout, redaction, and unsupported action.
- [ ] Update package/component/runtime READMEs and build, lint, and test for host
      and `wasm32-unknown-unknown`.

### Milestone 14: Runtime, Sandbox, and Repository Integration

- [ ] Run each read tool and each write preflight/commit pair through the real
      Wasm component with `WasmHostMock`, asserting exact provider traffic,
      protected injections, proposal hashes, side-effect count, idempotency,
      normalized output, and redaction.
- [ ] Add package tests for identity, scopes, two-provider OAuth, account
      bindings, permissions, tools, schemas, approval policy, host allowlists,
      fraud headers, limits, assets, and docs.
- [ ] Add Xero component/runtime manifests to `xtask/src/check.rs` and strengthen
      inventory tests so future app packages cannot be omitted silently.
- [ ] Update root and `apps/README.md` catalog docs and local commands; add only
      the reviewed Xero/HMRC Secret Manager deployment values.
- [ ] Run all package validation/build/test commands and the repository audit.

### Milestone 15: Authorised Provider Smoke Tests

- [ ] Connect a Xero Demo Company in a nominated non-production Firna workspace;
      verify organisation selection and cross-tenant denial with one and
      multiple authorised organisations.
- [ ] Smoke every read tool and every write lifecycle using disposable Demo
      Company records: draft, approve/post, bank transaction, transfer, payment,
      reversal/void/delete, stale proposal, duplicate invocation, ambiguous
      response, refresh, and audit receipt.
- [ ] Connect an HMRC sandbox organisation, verify VRN binding and fraud headers,
      prepare supported cash/accrual fixture returns, exercise every error
      scenario, submit one open sandbox obligation, and compare the filed return
      and receipt.
- [ ] Do not use a real ledger or make a real VAT filing as a generic smoke test.
      Any HMRC-required live submission needs a separately nominated business,
      production credentials, written filer approval, the completed tax review,
      and the exact production declaration journey.
- [ ] Record only redacted outcomes and current provider-limit usage in the PR.

### Milestone 16: Complete Repository Gate

- [ ] Run `cargo fmt --all -- --check`, every standalone component/runtime
      format and clippy command, Rust file-length lint/audit, and all relevant
      tests with a 100% pass rate.
- [ ] Run `cargo xtask check` and resolve every compile, lint, package, workflow,
      documentation, or test failure before proceeding.
- [ ] Inspect `git diff --check`, the complete diff, name-status, and deletion
      diff against `origin/main`; stop for any unapproved mainline removal.

### Milestone 17: Commit and Push Checked Work

- [ ] Fetch `origin/main`, capture the source tip, audit mainline additions from
      the merge base, and resolve overlaps path by path while preserving every
      mainline feature.
- [ ] Run `git add -A`, commit every package file, asset, lockfile, test, and
      document using a Conventional Commit title at most 50 characters (for
      example `feat(xero): add accounting and VAT tools`), and push the current
      branch without renaming it.
- [ ] Recheck committed name-status and deletion diff against `origin/main`.

### Milestone 18: Post-Push Review

- [ ] After the push, run `cargo xtask review` against `origin/main`.
- [ ] Do not automatically fix findings. Report each as a numbered item with
      severity, feature/codebase context, impact of doing nothing, lettered
      solution options, and a recommended option.
- [ ] When every preceding TODO is complete and review is reported, move this
      plan from Active to Completed in `plans/README.md`.

## Completion Criteria

- A workspace admin can connect and bind one Xero organisation, optionally bind
  the matching HMRC VAT account, and reconnect either provider independently.
- Agents can read live core accounting data and propose the documented Xero
  writes. No provider mutation happens until an authorised human approves the
  exact immutable proposal, and every result is idempotent, reconciled, and
  audited.
- Spend/receive-money bank transactions, transfers, payments, invoices, bills,
  credit notes, contacts, accounts, and manual journals complete their supported
  create/update/transition lifecycles against a Xero Demo Company.
- For supported UK VAT schemes, Firna deterministically derives all nine boxes
  from a complete Xero snapshot, preserves provenance and adjustments, obtains
  the human declaration, files through HMRC's production-approved VAT API, and
  stores a verified receipt. Unsupported schemes cannot be submitted.
- Unit, Wasm runtime, platform OAuth/account, approval/replay, cross-tenant,
  tax-golden, fraud-header, UI, package, sandbox, and full repository checks all
  pass.
- The checked branch is committed and pushed, post-push review findings are
  reported without silent fixes, and Xero/HMRC production approvals support the
  advertised rollout before catalog release.
