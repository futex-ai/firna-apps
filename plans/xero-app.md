# Xero Accounting App

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-03

## Outcome

Add `apps/xero`, an explicitly installed first-party app that mirrors the
capabilities available to an ordinary Xero Web app through the supported public
Accounting API. A workspace admin connects a workspace-owned Xero OAuth grant.
OAuth determines the organisations the app can access. Agents list that
authorised set and select one opaque organisation reference on each Accounting
tool call. The installation's app-level access decision governs the complete
Xero tool set. A provider write happens only after a human approves the exact,
immutable proposal for the selected organisation.

The app is generated and tested against a pinned official Xero OpenAPI
revision. When Xero adds, removes, deprecates, or changes an Accounting API
operation, the repository audit reports the drift and requires an explicit
contract update. Firna does not imitate Xero web-product features that Xero
does not expose through the supported public API.

## Supported API Boundary

The initial contract is pinned to the official **Xero Accounting API OpenAPI
16.1.0** at XeroAPI/Xero-OpenAPI commit
`45ab7e8ceccbbbfb41a0487a47f9d1d00cbb4a0f`. The pinned
`xero_accounting.yaml` SHA-256 is
`dc231cafd8de7d93a1c0c8cc26944724a8af5f1e59cc54b684187ee22df36318`.

“Supported” means an active Accounting API operation documented for ordinary
OAuth Web apps and reachable with generally available Accounting API scopes,
plus the official `/connections` endpoint required to list the organisations
authorised by OAuth. The app covers the capability even when Firna deliberately
exposes a safer single-record or typed-filter interface instead of Xero's bulk
wrapper or raw `where` expression.

### Included resource families

| Area | Supported resource families |
| --- | --- |
| Settings | Accounts, Branding Themes, Currencies, Invoice Reminders, Items, Organisation, Setup, Tax Rates, Tracking Categories, Users |
| Contacts | Contacts, Contact Groups, contact CIS settings |
| Invoices | Credit Notes, Invoices, Linked Transactions, Quotes, Purchase Orders, Repeating Invoices |
| Payments | Batch Payments, Overpayments, Payments, Prepayments |
| Banking records | Bank Transactions and Bank Transfers |
| Journals | Manual Journals |
| Files on records | Accounting API attachment list/download/upload/replace operations for every supported parent type |
| Collaboration | History retrieval and note creation wherever the Accounting API exposes them |
| Documents | Invoice, credit-note, quote, and purchase-order PDF retrieval; online-invoice lookup; invoice email |
| Budgets | Budget list and detail |
| Reports | Aged Payables, Aged Receivables, Balance Sheet, Bank Summary, Budget Summary, Executive Summary, Profit and Loss, Trial Balance, US 1099, and published AU BAS/NZ GST reports |

Region-, edition-, role-, feature-, and provider-state restrictions remain
provider-enforced and are normalized into typed availability or access
errors. A region-specific capability is visible only when the connected
organisation can use it.

### Explicit exclusions

The following are out of scope because they are not supported for this app
through the ordinary public Accounting API boundary:

- UK VAT-return preparation or submission. Xero exposes no Accounting API
  operation for it; users complete that workflow in Xero.
- Bank-statement lines, statement import, matching, cash coding, and
  reconciliation. Xero does not expose them through the Accounting API.
- Bank Feeds, Payment Services, Finance API, and Practice Manager operations,
  which require separate certification or commercial access.
- Provider-generated Journals, which require Xero Advanced tier, a security
  assessment, and use-case approval.
- Deprecated Expense Claims and Receipts.
- Payroll, Projects, Files-library, Assets, eInvoicing, and other separate Xero
  APIs.
- Webhooks, marketplace billing, custom connections, and non-tenanted APIs
  other than the `/connections` discovery required for organisation routing.
- Browser automation, undocumented endpoints, arbitrary provider URLs,
  arbitrary HTTP methods, raw OAuth credentials, and caller-supplied Xero
  tenant or connection ids.

These exclusions are product behavior, not deferred implementation TODOs. A
future expansion requires its own protocol and plan.

## Product Contract

### Installation and organisation routing

- Install policy is `explicit`.
- Xero uses a confidential authorization-code Web App with `offline_access`.
- Firna requests the current granular Accounting scopes required by enabled
  tool families; deprecated broad transaction/report scopes are forbidden.
- After token exchange, trusted host code calls
  `GET https://api.xero.com/connections`, retains
  `tenantType = ORGANISATION`, and stores the authorised set in a private
  connection registry for the current OAuth revision.
- `xero_organisations_list` returns the authorised organisations with only an
  opaque installation-scoped `organisation_ref`, display name, and safe
  connection timestamps. It never returns raw tenant or connection ids.
- Every tenant-scoped tool requires `organisation_ref`. Trusted host code
  validates it against the current grant, resolves the private tenant id, and
  injects `xero-tenant-id`; callers and the component cannot override it.
- Reconnect rebuilds the authorised set and invalidates every pending proposal.
  A removed or stale organisation ref fails before Accounting API dispatch.
- Access tokens, rotating refresh tokens, client secrets, tenant ids, and raw
  connection payloads never enter agent context, browser JSON, analytics,
  traces, logs, or app storage.
- A live installation and the platform's app-level access decision govern the
  whole Xero app. Xero adds no tool-specific role, permission, or action-level
  privilege layer.

### OAuth scopes

The manifest uses `offline_access` and the current granular scopes:

- `accounting.settings`, `accounting.contacts`,
  `accounting.invoices`, `accounting.payments`,
  `accounting.banktransactions`, and `accounting.manualjournals`
- `accounting.attachments` and `accounting.budgets.read`
- `accounting.reports.aged.read`,
  `accounting.reports.balancesheet.read`,
  `accounting.reports.banksummary.read`,
  `accounting.reports.budgetsummary.read`,
  `accounting.reports.executivesummary.read`,
  `accounting.reports.profitandloss.read`,
  `accounting.reports.trialbalance.read`,
  `accounting.reports.taxreports.read`, and
  `accounting.reports.tenninetynine.read`

Scopes are additive in Xero. Firna shows the complete requested set before
connection and requires reconnect when Xero did not grant a required scope.

### Read contract

- Every model-visible input and output is a closed JSON Schema generated from
  the pinned provider contract plus Firna overrides.
- The host-backed `xero_organisations_list` tool is handwritten because
  `/connections` is outside the Accounting OpenAPI; coverage requires exactly
  this one additional discovery tool.
- Reads use a typed operation and typed parameters; tools never accept raw
  `where`, `order`, URL, path, method, headers, or provider JSON.
- Lists fetch exactly one bounded page. Unpaged provider operations fail rather
  than truncate if their normalized response exceeds the declared item or
  1 MiB output cap.
- Provider ids are preserved where needed for follow-up operations. OAuth
  credentials, full bank-account numbers, tax identifiers, private contact
  details not required by the requested operation, provider URLs, and raw
  validation bodies are redacted.
- Binary downloads are returned as authenticated Firna attachment artifacts,
  not base64 in model context. Uploads accept an existing Firna attachment id
  and enforce Xero's file-name, media-type, and size limits.
- Region-specific reports return a typed unavailable result when the connected
  organisation is ineligible; Firna never fabricates report data.

### Write and approval contract

- Every Xero mutation is `external_write` with
  `approval.mode = human_required`. No amount, schedule, agent role, prior
  approval, or workspace policy can auto-approve it.
- `human_required` is an execution policy, not a permission. The ordinary
  human-prompt access rules determine who may resolve a proposal; Xero adds no
  owner/admin gate or separate financial-write grant.
- A read-only preflight resolves current Xero state, provider names, lock dates,
  references, totals, validation rules, and the exact provider operation.
- Firna stores a canonical proposal bound to the workspace, installation,
  organisation ref, private tenant-binding fingerprint, authorization
  revision, app/tool version, normalized payload, provider ids and versions,
  proposer, expiry, and resolver context. Approval expires after 15 minutes.
- Editing any field or observing changed provider state creates a new proposal.
- Trusted host code injects a single-use approval receipt and
  `Idempotency-Key` only for commit. Neither is a model/component input.
- Provider bulk endpoints are invoked with one logical record unless the
  provider resource is itself a batch object, such as Batch Payment.
- Destructive transitions, Setup, invoice email, notes, attachment replacement,
  and immediately-authorised banking/payment operations clearly state their
  consequence in approval UI.
- Ambiguous timeouts are reconciled by exact provider reads before any retry.
  Firna never changes an idempotency key merely because the response was lost.
- Proposal, decision, request fingerprint, provider result, reconciliation,
  actor, app-access checks, and approval checks are immutable redacted audit
  records.

### API drift and generation

- The repository vendors the exact official OpenAPI file and its MIT licence.
- A checked-in policy file classifies every provider operation as
  `read`, `write`, `deprecated`, `certification_required`, or
  `unsupported_separate_api`.
- Generation fails on an unclassified path/method, duplicate operation id,
  unsupported schema construct, missing bound, or changed enum.
- Generated Rust models, operation registry, manifest schema fragments, and
  conformance fixtures are committed. Handwritten code owns normalization,
  redaction, approval summaries, provider-state validation, and error mapping.
- CI compares the vendored source metadata and generated outputs. Updating the
  OpenAPI file without regenerating, or changing generated output by hand,
  fails the repository audit.
- Provider documentation can be stricter or newer than OpenAPI metadata.
  Explicit policy overrides record those cases, including deprecated
  Expense Claims/Receipts and certification-only Payment Services.

## Platform Dependencies

The pinned Firna platform revision cannot yet provide the complete contract:

1. `standard_oauth2` does not persist expiry metadata, execute rotating-token
   refresh safely, serialize concurrent refresh, or maintain Xero's inactivity
   window.
2. OAuth completion cannot persist the complete authorised `/connections`
   registry or expose its safe projection as `xero_organisations_list`.
3. Tool invocation cannot resolve an installation-scoped `organisation_ref`
   and inject its tenant header while rejecting component/model overrides.
4. `external_write` is descriptive metadata rather than an immutable
   preflight/approval/commit boundary.
5. There is no durable proposal, idempotency, reconciliation, or financial
   audit ledger.
6. Binary Accounting API attachments need an authenticated artifact bridge
   rather than model-visible base64 payloads.

These provider-neutral capabilities must land in the platform before the Xero
package exposes affected tools.

## Tool Architecture

The app groups related provider operations without exposing a generic HTTP
escape hatch:

- one typed read tool per resource family;
- one host-backed `xero_organisations_list` tool for OAuth-authorised
  organisation discovery;
- one typed mutation tool per resource family with a discriminated action
  schema;
- shared typed attachment and history tools whose parent-kind enum contains
  only provider-supported parents;
- one typed report tool with a closed report-kind union;
- one document tool for supported PDF and online-invoice reads;
- one invoice-email mutation;
- one Setup mutation isolated behind the strongest approval treatment.

Each action maps to one classified path/method/operation id in the generated
registry. Multiple semantic actions may share an operation only through an
explicit policy row. Tests prove every included provider operation is reachable
and every excluded operation is unreachable.

## Milestones

### Milestone 1: Re-scope and Complete the Xero Protocols

Create the implementable Accounting API contracts before code.

- [x] Verify the supported boundary against the current official Xero
      Accounting API overview, scopes page, endpoint documentation, and OpenAPI
      16.1.0; pin the upstream commit and file checksum.
- [x] Replace the VAT/HMRC protocol with Xero-only app, operation-inventory,
      schema-generation, read, write, attachment, history, document, OAuth,
      redaction, and error contracts, keeping each protocol near 250 lines.
- [x] Classify all 235 OpenAPI operations and explicitly override deprecated
      Expense Claims/Receipts, gated Journals, and certification-only Payment
      Services.
- [ ] Specify every included action's method/path, scope, region/role
      restriction, input/output schema source, pagination, response cap,
      preflight, approval, idempotency, reconciliation, and audit behavior.
- [x] Resolve the prior review by removing the withdrawn HMRC findings,
      bounding every recursive array, and distinguishing component runtime
      hosts from trusted OAuth endpoints.
- [x] Validate Markdown, local/external links, OpenAPI inventory coverage, and
      diffs; commit and push the platform protocol revision, then run its
      required post-push review without automatically fixing findings.

Protocol evidence: the replacement contracts were committed and pushed on
`calummoore/xero-platform-support` at `f925cab8a`. Markdown, link, diff, and
235-operation coverage validation passed. The required post-push review
completed successfully and confirmed the original 214 included plus 21 excluded
operation inventory, while raising the unresolved items below. The
user-selected organisation-routing revision was committed and pushed at
`748064d89`; its follow-up post-push review no longer reported the missing
selection continuation or tenant-id exposure findings and raised three new
items recorded below. The user-selected app-level-access revision was committed
at `6be368dca`, merged with the latest platform main, and pushed at
`9d82a1395`; its follow-up review no longer reported the financial-write
permission finding, repeated Important 6, Important 7, and Minor 1, and raised
Important 8, Important 9, and Minor 2 below. The complete recommended
resolution set was applied in platform commit `ddb8865f4`, merged with the
latest target branch, and pushed at `7fc5ceb15` before implementation began.

#### Post-push protocol review findings

The user's instruction to implement the complete plan authorises the recorded
recommended resolutions. Each item is checked only after the corresponding
platform protocol text is aligned.

- [x] **Important 1 — authorised organisation routing:** apply the user's
      selected resolution: keep OAuth as the authority boundary, add the
      model-visible `xero_organisations_list` tool, and require its opaque
      `organisation_ref` on every tenant-scoped tool without adding an
      install-time selection continuation. The follow-up post-push review did
      not re-report the original continuation finding.
- [x] **Important 2 — financial-write approval contract:** apply the user's
      selected resolution: keep `human_required` as a mutation-execution
      policy, use ordinary human-prompt resolution, and make app-level access
      the sole Xero tool-access boundary. Do not add an owner/admin gate, a
      financial-write grant, or any tool/action-specific permission. The
      follow-up review did not re-report this finding.
- [x] **Important 3 — tenant-id confidentiality:** keep raw Xero connection and
      tenant ids in private credential/install metadata instead of the existing
      public `provider_account_id`; expose only a reviewed display label and an
      opaque organisation ref. The follow-up review did not re-report the
      original exposure finding.
- [x] **Important 4 — operation/action mapping:** use a tested many-to-one
      mapping: every semantic action maps to exactly one provider operation,
      every included operation maps to at least one classified action, and
      multiple actions may share an operation only through explicit policy.
- [x] **Important 5 — token versus authorization revisions:** separate routine
      access/refresh-token rotation from the authorization revision so a
      30-minute token refresh does not retire organisation refs or invalidate a
      valid 15-minute write proposal. Bump authorization only for reconnect,
      scope, identity, or authorised-connection-set changes.
- [x] **Important 6 — gated Journals access:** classify the three
      `accounting.journals.read` operations as `certification_required`,
      reducing the ordinary V1 inventory to 211 included and 24 excluded until
      Xero grants Advanced-tier, security-assessment, and use-case approval.
- [x] **Important 7 — artifact result envelopes:** add
      `organisation: { organisation_ref, name }` to the exact attachment and
      PDF artifact results so they satisfy the shared tenant-scoped output
      contract and generated schemas cannot diverge.
- [x] **Important 8 — disconnected-organisation error precedence:** return
      installation-level `auth_required` when no authorised organisation
      remains; when at least one remains, return `organisation_disconnected`
      for stale, disconnected, foreign, or invented refs.
- [x] **Important 9 — action discriminant casing:** use lower snake case for
      every JSON action discriminant while retaining provider enum casing for
      non-action values.
- [x] **Minor 1 — planned-page lifecycle:** link every planned Xero protocol
      page to this active plan and state concrete criteria for changing its
      status from planned to current.
- [x] **Minor 2 — online-invoice handoff hosts:** allow authenticated
      online-invoice handoff only to HTTPS `in.xero.com`, with no userinfo,
      non-default port, IP literal, or alternative host.

### Milestone 2: Platform OAuth and Tenant Binding

Implement provider-neutral OAuth support in `futex-ai/firna`. No UI or
mockup work belongs here.

- [ ] Persist typed access/refresh expiry metadata and implement atomic
      single-use refresh-token rotation with one refresh in flight per grant.
- [ ] Implement Xero refresh timing, the documented old-token ambiguity grace
      behavior, inactivity expiry, stable terminal states, and redacted errors.
- [ ] Persist the complete current `tenantType = ORGANISATION` connection
      registry per grant revision and expose only its safe projection through
      `xero_organisations_list`, including empty and disconnected states.
- [ ] Require `organisation_ref` on every tenant-scoped tool, resolve it only
      inside the current workspace/install/grant registry, inject the trusted
      `xero-tenant-id`, and reject every raw-id or header override.
- [ ] Put clocks, OAuth HTTP, discovery, vault, store, and transaction behavior
      behind traits; add `unimock` concurrency, cross-tenant, refresh,
      redaction, expiry, revoke, reconnect, disable, and uninstall tests.
- [ ] Update protocols and crate READMEs; run full platform checks, commit,
      push, and review.

### Milestone 3: Platform Financial Write Controls

Add the reusable mutation boundary required by Xero.

- [ ] Add manifest-declared `human_required` approval for
      `external_write`, with no automatic policy path and no separate
      tool/action permission layer.
- [ ] Add preflight/commit component exports and durable canonical proposals
      bound to tool/version, workspace, organisation ref, private tenant
      fingerprint, authorization revision, payload, current provider versions,
      proposer, resolver, and expiry.
- [ ] Inject and atomically consume a single-use approval receipt and provider
      idempotency key; reject replay, payload changes, stale state, and
      app-access or resolver-access changes.
- [ ] Add pending, approved, executing, succeeded, rejected, expired, stale,
      ambiguous, reconciled, and failed mutation states that survive restarts
      and concurrent workers.
- [ ] Add provider read-after-timeout reconciliation and immutable redacted
      financial audit events.
- [ ] Add service, store, migration, RPC, agent-tool, and adversarial tests;
      update protocols/READMEs, run full checks, commit, push, and review.

### Milestone 4: Platform Attachment Artifact Bridge

Support Accounting API file operations without placing binary data in agent
context. No UI or mockup work belongs here.

- [ ] Add a capability-scoped app import that streams one authenticated Firna
      attachment artifact to a reviewed provider request with exact byte,
      filename, media-type, timeout, and destination limits.
- [ ] Add a provider-response path that stores an allowed binary response as a
      workspace-scoped Firna artifact and returns only safe metadata plus an
      authenticated artifact reference.
- [ ] Reject redirects, content-type confusion, oversized files, cross-workspace
      ids, credential/header overrides, and component-selected hosts.
- [ ] Cover upload, replace, download, cancellation, failure cleanup,
      redaction, and authorization with unit and integration tests.
- [ ] Update protocols/READMEs, run full checks, commit, push, and review.

### Milestone 5: Connection and Approval Mockups

Tags: mockup

Specify product journeys before UI implementation.

- [ ] Extend Installed Apps with mobile and desktop screens for Xero consent,
      authorised-organisation list/empty states, connected capability groups,
      missing scopes, reconnect-required, revoked, disabled, and uninstalled
      states.
- [ ] Add standalone mobile/desktop screens for ordinary edits, destructive
      transitions, immediately-authorised bank/payment records, Setup, invoice
      email, history note, attachment upload/replace, stale proposal, provider
      validation failure, success, and ambiguous reconciliation.
- [ ] Keep screens in the existing hierarchy, use shared components, render no
      more than five screens per screen-spec page, and compose flows only from
      linked standalone screen components.
- [ ] Use plain product language and real-shaped empty/error states; never show
      tenant ids, OAuth tokens/scopes as engineering jargon, raw provider
      errors, test labels, or invented business data.
- [ ] Run mockup build/check/test/typecheck and direct-file visual smoke tests;
      commit generated HTML with its React source.

### Milestone 6: Connection and Approval UI

Tags: ui

Implement the approved designs using real backend state.

- [ ] Render OAuth, authorised-organisation inventory, granted-capability,
      reconnect, disable, and uninstall states with accessible
      loading/empty/error paths.
- [ ] Render server-owned immutable proposal details and provider-resolved
      names; the client cannot alter hidden payloads or manufacture receipts.
- [ ] Use the ordinary authenticated human-prompt resolver for every write and
      support reject, cancel, expiry, stale, ambiguous, reconciled, and failed
      states on mobile and web.
- [ ] Add authenticated attachment upload/download handoff without exposing
      raw provider credentials or binary content to agent messages.
- [ ] Cover callback restoration, cross-workspace isolation, revoked roles,
      double clicks, stale tabs, accessibility, and resumed agent turns.
- [ ] Run relevant format, lint, type, test, build, and smoke checks; commit,
      push, and review in the platform repository.

### Milestone 7: Adopt the Supporting Platform Revision

- [ ] After platform work merges, update `platform.toml`, deployment workflow
      pins, every standalone runtime-test manifest/lockfile, and the root CLI
      install revision together.
- [ ] Add compatibility coverage that rejects a partial pin update.
- [ ] Run `cargo xtask check` and keep every existing app green before adding
      the Xero package.

### Milestone 8: Provision the Xero Web App

- [ ] Create the production confidential Xero Web App named `Firna` with the
      exact callback `https://firna.ai/oauth/xero/callback`.
- [ ] Request only the reviewed generally available granular scopes and confirm
      the connection tier needed for rollout; do not request gated Journals.
- [ ] Put the public client id in manifest environment and transfer the client
      secret directly to `firna-prod-app-xero-client-secret` in Google Secret
      Manager; never place it in Git, `.context`, shell history, logs,
      screenshots, fixtures, or chat.
- [ ] Record only redacted app id, callback, scopes, tier, collaborators,
      approvals, and secret version in the implementation handoff.

### Milestone 9: Generate the Xero Package Contract

- [ ] Scaffold `apps/xero` as a standalone component and runtime-test
      workspace with committed lockfiles, required READMEs, and approved Xero
      brand assets.
- [ ] Vendor OpenAPI 16.1.0 and its MIT licence; add the reviewed operation
      policy and deterministic generator for Rust models, operation registry,
      manifest schemas, and conformance fixtures.
- [ ] Generate a `1.0.0` manifest with explicit install, reviewed granular
      scopes, runtime/OAuth hosts, read/write approval metadata, limits, and no
      ingress/events or restricted APIs.
- [ ] Add drift, determinism, operation reachability, schema closure, enum,
      scope, exclusion, and generated-file audit tests.
- [ ] Build and test the generator and generated package on host and
      `wasm32-unknown-unknown`.

### Milestone 10: Implement Supported Read Operations

- [ ] Implement typed provider transport, request mapping, response decoding,
      pagination, normalization, redaction, region/role availability, and
      stable errors for every included GET operation.
- [ ] Implement authenticated artifact results for supported PDFs and
      attachments, plus typed online-invoice, CIS, organisation-action,
      budget, manual-journal, history, and regional-report reads.
- [ ] Put provider HTTP, clock, and artifact behavior behind traits and keep
      production Rust files below 300 lines.
- [ ] Add external `_tests_` modules with `unimock`, conformance fixtures,
      malformed-response, boundary, redaction, and operation-coverage tests.
- [ ] Run component format, clippy, host tests, Wasm build, and runtime read
      smoke tests.

### Milestone 11: Implement Supported Write Operations

- [ ] Implement preflight and commit mappings for every included POST, PUT,
      PATCH, and DELETE operation using one logical record per invocation.
- [ ] Implement provider status/role/region/lock-date validation, decimal
      handling, live tax/account/tracking references, and provider-calculated
      total verification.
- [ ] Implement attachment upload/replace, history notes, invoice email, Setup,
      allocations, group membership, status transitions, and all other
      supported action unions.
- [ ] Implement idempotency and exact read-after-timeout reconciliation for
      each mutation family.
- [ ] Add `unimock`, conformance, proposal-summary, stale-state, app-access,
      replay, timeout, redaction, and full operation-coverage tests.
- [ ] Run component format, clippy, host tests, Wasm build, and runtime
      preflight/commit smoke tests.

### Milestone 12: Repository and Runtime Integration

- [ ] Exercise every tool family through the real Wasm component with
      `WasmHostMock`, asserting exact provider traffic, organisation-ref
      resolution, tenant injection, side-effect count, proposal hash,
      idempotency, output, and redaction.
- [ ] Add package tests for identity, scopes, OAuth, organisation routing,
      app-level access, tool schemas, approval policy, host allowlists, limits,
      operation inventory, exclusions, assets, and docs.
- [ ] Add Xero component/runtime manifests to `xtask/src/check.rs` and make
      app inventory auditing reject silently omitted packages.
- [ ] Update root and `apps/README.md` catalog documentation, commands, and
      only the reviewed Xero Secret Manager deployment value.
- [ ] Run all package validation/build/test commands and repository audit.

### Milestone 13: Authorised Xero Demo-Company Smoke Tests

- [ ] Connect a nominated Xero Demo Company in a non-production Firna workspace
      and verify `xero_organisations_list` plus per-tool routing with one and
      multiple authorised organisations, stale refs, and cross-tenant denial.
- [ ] Smoke every read family and every applicable regional/role unavailable
      path without using or copying a real ledger.
- [ ] Smoke every write action with disposable records, including approval,
      stale proposal, duplicate invocation, ambiguous response, refresh, and
      audit receipt.
- [ ] Confirm unsupported, deprecated, certification-only, and separate-API
      operations, including gated Journals, are unreachable.
- [ ] Record only redacted outcomes and current provider-limit usage in the PR.

### Milestone 14: Complete Repository Gate

- [ ] Run `cargo fmt --all -- --check`, every standalone component/runtime
      format and clippy command, Rust file-length lint/audit, and all relevant
      tests with a 100% pass rate.
- [ ] Run `cargo xtask check` and resolve every compile, lint, package,
      workflow, documentation, or test failure before proceeding.
- [ ] Inspect `git diff --check`, the complete diff, name-status, and deletion
      diff against `origin/main`; stop for any unapproved mainline removal.

### Milestone 15: Commit and Push Checked Work

- [ ] Fetch `origin/main`, capture the source tip, audit mainline additions
      from the merge base, and preserve every mainline feature.
- [ ] Run `git add -A`, commit every package file, generated file, asset,
      lockfile, test, and document with a Conventional Commit title no longer
      than 50 characters, then push the current branch without renaming it.
- [ ] Recheck committed name-status and deletion diff against `origin/main`.

### Milestone 16: Post-Push Review

- [ ] After the push, run `cargo xtask review` against `origin/main`.
- [ ] Do not automatically fix findings. Report each as a numbered item with
      severity, codebase/feature context, impact of doing nothing, lettered
      solution options, and a recommended option.
- [ ] When every preceding TODO is complete and review is reported, move this
      plan from Active to Completed in `plans/README.md`.

## Completion Criteria

- A workspace admin can connect Xero with the current granular Accounting API
  scopes; agents can list only OAuth-authorised organisations and select one
  validated opaque reference per tool; reconnect, disable, and uninstall are
  safe; one app-level access decision applies to the complete Xero tool set.
- Every active, ordinary-Web-app Accounting API capability classified in the
  pinned contract is reachable through a closed typed Firna tool, and every
  excluded capability is unreachable.
- Every provider mutation requires approval of an immutable preflight, uses
  durable idempotency and reconciliation, and produces a redacted audit trail.
- Attachments and PDFs use authenticated artifacts rather than model-visible
  binary data; sensitive provider, bank, tax, and credential values stay behind
  trusted boundaries.
- Deprecated Expense Claims/Receipts, gated Journals, certification-only
  Payment Services and Bank Feeds, bank reconciliation, VAT submission, and
  separate Xero APIs are neither advertised nor callable.
- Generated artifacts match the pinned OpenAPI and policy exactly; all unit,
  runtime, platform, UI, package, demo-company, and repository checks pass.
- Checked work is committed and pushed, and post-push review findings are
  reported before the plan is marked complete.
