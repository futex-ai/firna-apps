# Xero Accounting App

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-04

## Outcome

Add `apps/xero`, an explicitly installed first-party app that mirrors the
capabilities available to an ordinary Xero Web app through the supported public
Accounting API. A workspace admin connects a workspace-owned Xero OAuth grant.
OAuth determines the organisations the app can access. Agents list that
authorised set and select one opaque organisation reference on each Accounting
tool call. The installation's app-level access decision governs the complete
Xero tool set. Read and write tools use the platform's ordinary app-execution
path: an authorised call executes without a Xero-specific confirmation prompt,
approval proposal, or action-level permission.

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
- Reconnect rebuilds the authorised set and invalidates prior organisation
  references. A removed or stale organisation ref fails before Accounting API
  dispatch.
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

Xero's [official scope guide](https://developer.xero.com/documentation/guides/oauth2/scopes/)
states that every new and existing Web or PKCE app has been assigned granular
scopes since March 2026. The replaced broad transaction and report scopes are
deprecated and remain only for legacy connections until September 2027. Firna
is a new Web app created after the granular-scope rollout, so its manifest must
use the granular scopes above and must not request the deprecated broad scopes.

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

### Write contract

- Xero mutations use the same execution path as mutations in an ordinary
  installed app. They do not declare `approval.mode = human_required` and do
  not create a human prompt, approval proposal, or approval receipt.
- OAuth scopes, the live installation, the platform's app-level access
  decision, and Xero's own organisation/role/state checks are the complete
  authority boundary. Xero adds no owner/admin gate or tool/action-specific
  permission.
- Each invocation validates its typed input, resolves current Xero state when
  required by the provider operation, validates `organisation_ref` immediately
  before dispatch, and executes the selected operation directly.
- Trusted host code owns credential and tenant-header injection. Neither raw
  OAuth credentials nor raw tenant/header values are model or component inputs.
- Provider bulk endpoints are invoked with one logical record unless the
  provider resource is itself a batch object, such as Batch Payment.
- Destructive transitions, Setup, invoice email, notes, attachment replacement,
  and immediately effective banking/payment operations clearly state their
  consequence in tool descriptions and results, without adding a confirmation
  step.
- A trusted idempotency key is stable across transport retries of one
  invocation and is never accepted as model input.
- Ambiguous timeouts are reconciled by exact provider reads before any retry.
  Firna never changes an idempotency key merely because the response was lost.
- Tool invocation, request fingerprint, selected organisation, provider result,
  reconciliation outcome, actor, and app-access checks use the platform's
  ordinary redacted audit path.

### API drift and generation

- The repository vendors the exact official OpenAPI file and its MIT licence.
- A checked-in policy file classifies every provider operation as
  `read`, `write`, `deprecated`, `certification_required`, or
  `unsupported_separate_api`.
- Generation fails on an unclassified path/method, duplicate operation id,
  unsupported schema construct, missing bound, or changed enum.
- Generated Rust models, operation registry, manifest schema fragments, and
  conformance fixtures are committed. Handwritten code owns normalization,
  redaction, provider-state validation, and error mapping.
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
4. Standard direct-write execution must preserve a trusted organisation
   binding, idempotency across transport retry, reconciliation, and redacted
   audit behavior without introducing a human-approval boundary.
5. Binary Accounting API attachments need an authenticated artifact bridge
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
- one typed Setup mutation using the same direct execution contract as other
  writes.

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
      validation, execution, idempotency, reconciliation, and audit behavior.
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
- [x] **Important 2 — standard write execution:** superseded on 2026-08-04 by
      the user's explicit product decision that Xero is a normal app. Xero does
      not declare `human_required`, publish approval prompts, or add an
      owner/admin, financial-write, tool, or action-specific permission. OAuth
      and app-level access remain the authority boundary.
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
      30-minute token refresh does not retire organisation refs or disrupt an
      in-flight provider request. Bump authorization only for reconnect, scope,
      identity, or authorised-connection-set changes.
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

- [x] Persist typed access/refresh expiry metadata and implement atomic
      single-use refresh-token rotation with one refresh in flight per grant.
- [x] Implement Xero refresh timing, the documented old-token ambiguity grace
      behavior, inactivity expiry, stable terminal states, and redacted errors.
- [x] Persist the complete current `tenantType = ORGANISATION` connection
      registry per grant revision and expose only its safe projection through
      `xero_organisations_list`, including empty and disconnected states.
- [x] Require `organisation_ref` on every tenant-scoped tool, resolve it only
      inside the current workspace/install/grant registry, inject the trusted
      `xero-tenant-id`, and reject every raw-id or header override.
- [x] Put clocks, OAuth HTTP, discovery, vault, store, and transaction behavior
      behind traits; add `unimock` concurrency, cross-tenant, refresh,
      redaction, expiry, revoke, reconnect, disable, and uninstall tests.
- [x] Update the migration inventory contract and checksum lock for the new
      forward-only OAuth lifecycle tables.
- [x] Update protocols and crate READMEs; run full platform checks, commit,
      push, and review.

Implementation evidence: the provider-neutral lifecycle was committed at
`3380573db`, integrated path-by-path with `origin/main` at `613a4fadf`, and
pushed on `calummoore/xero-platform-support`. The authoritative post-merge
`cargo xtask check` passed, including 2,934 Rust tests, 1,427 universal-app unit
tests, 104 universal-app smoke tests, web smoke tests, all target builds,
Terraform validation, and Helm validation. The branch had no deletions relative
to `origin/main`. The required post-push `cargo xtask review` completed and
raised the unresolved findings below; repository policy requires an explicit
user decision before any review-driven fix.

#### Post-push implementation review findings

- [ ] **Critical 1 — refresh error classification:** decide how to prevent a
      Xero token-endpoint 429 or ordinary non-2xx response from being treated
      as a rejected grant and destructively retiring otherwise valid local
      credentials and connections. Doing nothing can force unnecessary
      workspace reauthorization during transient throttling. Option A adds a
      typed rate-limited failure and preserves bounded `Retry-After`; option B
      makes only explicit `invalid_grant` terminal and maps all other 4xx
      failures non-destructively. **Recommendation: implement A and B, with
      terminal cleanup reserved for definitive invalid-grant semantics.**
- [ ] **Important 2 — lifecycle mapping invariants:** decide whether lifecycle
      token kinds must be distinct and mapped for every auth requirement using
      the flow. Doing nothing lets a reviewed manifest pass validation but fail
      OAuth completion or refresh, and can create ambiguous same-kind token
      rows. Option A strengthens validation per requirement; option B moves the
      lifecycle declaration under each auth requirement. **Recommendation: use
      A unless the product needs different lifecycle policy per requirement.**
- [ ] **Important 3 — uninstall transaction ordering:** decide how to keep a
      stale installation revision from revoking local OAuth state before the
      installation status CAS fails. Doing nothing can leave an installation
      marked live without usable credentials. Option A changes status before
      cleanup; option B atomically fences the installation revision and retires
      local lifecycle state in one store transaction, leaving provider
      revocation best-effort. **Recommendation: use B for durable local
      consistency.**
- [ ] **Important 4 — scoped-row normalization:** decide whether refresh and
      connection-replacement transactions must overwrite caller-supplied
      workspace, app, and installation fields from the locked grant and
      installation. Doing nothing allows a future faulty caller to delete
      current rows and then persist mismatched replacements or hit avoidable FK
      failures. Option A normalizes only in the current service; option B
      applies the initialization pattern inside every store transaction.
      **Recommendation: use B so the persistence boundary enforces scope.**
- [ ] **Minor 1 — connection field bounds:** decide explicit maximum lengths
      for provider connection ids, tenant ids, and model-visible organisation
      names. Doing nothing remains globally capped by the 1 MiB response limit
      but permits one excessively large field into storage and model context.
      Option A cap only the display name; option B bound every parsed field and
      add oversized-provider fixtures. **Recommendation: use B for a reusable
      provider-contract rule.**

### Former Milestone 3: Superseded Platform Approval Work

The provider-neutral `human_required` mutation boundary was implemented on the
platform branch before the 2026-08-04 product correction. It is not part of the
Xero contract, is not a prerequisite for this app, and must not be declared by
the Xero manifest or invoked by Xero tools. Approval-only branch changes must
be excluded when integrating the platform work for Xero, without deleting or
overriding any capability already present on `origin/main`.

Historical evidence: the optional boundary was committed at `16bb9302c`,
integrated at `473aae629`, checked, pushed, and reviewed. Its approval-queue,
proposal-binding, publishing, and approval-UI findings are not Xero work.

### Milestone 4: Platform Attachment Artifact Bridge

Support Accounting API file operations without placing binary data in agent
context. No UI or mockup work belongs here.

- [x] Add a capability-scoped app import that streams one authenticated Firna
      attachment artifact to a reviewed provider request with exact byte,
      filename, media-type, timeout, and destination limits.
- [x] Add a provider-response path that stores an allowed binary response as a
      workspace-scoped Firna artifact and returns only safe metadata plus an
      authenticated artifact reference.
- [x] Reject redirects, content-type confusion, oversized files, cross-workspace
      ids, credential/header overrides, and component-selected hosts.
- [x] Cover upload, replace, download, cancellation, failure cleanup,
      redaction, and authorization with unit and integration tests.
- [x] Update protocols/READMEs, run full checks, commit, push, and review.

Implementation evidence: the capability-gated attachment artifact bridge was
committed at `1b9998b64`, integrated path-by-path with `origin/main` at
`8eb1f325b`, and pushed on `calummoore/xero-platform-support`. The authoritative
post-merge `cargo xtask check` passed, including 2,984 Rust workspace tests,
1,436 universal-app unit tests, 104 universal-app smoke tests, 148 public-app
tests, 148 web tests, 18 web browser smoke tests with two expected skips, every
target build/export, mockup checks, Terraform validation, and Helm validation.
The required post-push `cargo xtask review` also reported findings in the
optional approval infrastructure included in the branch diff. Those findings
do not apply to Xero's standard direct-write path and are tracked outside this
plan.

### Milestone 5: Xero Connection Mockups

Tags: mockup

Specify the normal installed-app connection journey before UI implementation.

- [x] Extend Installed Apps with mobile and desktop screens for Xero consent,
      authorised-organisation list/empty states, connected capability groups,
      missing scopes, reconnect-required, revoked, disabled, and uninstalled
      states.
- [ ] Remove the superseded Xero approval, proposal, confirmation, and
      write-outcome screens and flows from React mockup source and generated
      HTML. Xero write tools use the ordinary agent tool experience and do not
      introduce app-specific write UI.
- [ ] Keep the remaining connection screens in the existing hierarchy, use
      shared components, render no more than five screens per screen-spec page,
      and compose flows only from linked standalone screen components.
- [ ] Use plain product language and real-shaped empty/error states; never show
      tenant ids, OAuth tokens/scopes as engineering jargon, raw provider
      errors, test labels, or invented business data.
- [ ] Run mockup build/check/test/typecheck and commit generated HTML with its
      React source.
- [ ] Run direct-file visual smoke tests for every changed mockup page. The
      required Conductor in-app Browser currently reports that no browser is
      available; retry as soon as a Browser tab is attached to this workspace.

Historical implementation evidence: the initial Xero connection, lifecycle,
approval, attachment, and outcome mockups were committed at `4772db5d6` on
`calummoore/xero-platform-support`. All 40 generated mobile/desktop HTML
artifacts were committed with their React screen components. Mockup build,
check, typecheck, and all 28 mockup tests passed. The authoritative
`cargo xtask check` also passed, including 2,984 Rust workspace tests, 1,436
universal-app unit tests, 104 universal-app smoke tests, 148 public-app tests,
148 web tests, 18 web browser smoke tests with two expected skips, every target
build/export, Terraform validation, and Helm validation. The approval-specific
artifacts are now superseded and must be removed before this milestone is
complete.

### Milestone 5A: Integrate OAuth and Standard Writes

Reconcile the merged provider-neutral OAuth refresh foundation with Xero's
multi-organisation routing before UI implementation. No UI or mockup work
belongs here. Milestone 6 cannot begin until this milestone is complete.

- [ ] Fetch and audit the latest platform `origin/main`, starting with OAuth
      refresh PR #1012 (`ad4072fc4`), and resolve every conflict path-by-path;
      preserve all upstream behavior, migrations, tests, and documentation.
- [ ] Use the merged platform token service as the sole owner of access-token
      refresh, rotating refresh credentials, proactive refresh, and exact-once
      401 replay; do not retain a parallel Xero token-refresh implementation.
- [ ] Retain the private Xero connection registry and opaque
      `organisation_ref` projection, adapting it to the merged installation
      credential lifecycle without exposing raw connection or tenant ids.
- [ ] Keep authorization revision independent from ordinary access/refresh
      token rotation: reconnect, scope, identity, or authorised-connection-set
      changes invalidate organisation refs, while routine token refresh does
      not.
- [ ] Exclude the Xero-driven `human_required`, durable-proposal,
      preflight/commit, approval-receipt, and human-prompt changes from the
      platform integration. Preserve all corresponding behavior already on
      `origin/main`; do not ship the branch-only approval framework for Xero.
- [ ] Validate the call's current `organisation_ref` inside the trusted host for
      every direct read, write, and attachment request, then inject the Xero
      tenant header from the private connection registry. Reject raw ids,
      headers, stale refs, and refs from another workspace or installation.
- [ ] Preserve the selected organisation across token refresh and transport
      retry, and preserve one trusted idempotency key across retries of the same
      direct write without creating an approval proposal.
- [ ] Add adversarial tests for changed, missing, duplicated, and foreign
      organisation/auth bindings, including direct writes, reconciliation,
      artifact upload, proactive refresh, and exact 401 replay paths. Confirm
      that Xero writes dispatch without a human-prompt record.
- [ ] Update OAuth, mutation, artifact, and Xero protocols plus affected crate
      READMEs; run full platform checks, commit and push the integrated branch,
      then run the required post-push review.

### Milestone 6: Xero Connection and Organisations UI

Tags: ui

Implement the connection designs using real backend state.

- [ ] Render OAuth, authorised-organisation inventory, granted-capability,
      reconnect, disable, and uninstall states with accessible
      loading/empty/error paths.
- [ ] Do not add Xero-specific approval, proposal, or write-confirmation UI;
      direct writes use the ordinary agent tool flow and ordinary tool results.
- [ ] Add authenticated attachment upload/download handoff without exposing
      raw provider credentials or binary content to agent messages.
- [ ] Cover callback restoration, cross-workspace isolation, revoked app access,
      double clicks, stale tabs, reconnect, and accessibility.
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
      provider-review status, and secret version in the implementation handoff.

### Milestone 9: Generate the Xero Package Contract

- [ ] Scaffold `apps/xero` as a standalone component and runtime-test
      workspace with committed lockfiles, required READMEs, and approved Xero
      brand assets.
- [ ] Vendor OpenAPI 16.1.0 and its MIT licence; add the reviewed operation
      policy and deterministic generator for Rust models, operation registry,
      manifest schemas, and conformance fixtures.
- [ ] Generate a `1.0.0` manifest with explicit install, reviewed granular
      scopes, runtime/OAuth hosts, read/write effect metadata, limits, no
      `human_required` approval declaration, and no ingress/events or restricted
      APIs.
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

- [ ] Implement direct execution mappings for every included POST, PUT, PATCH,
      and DELETE operation using one logical record per invocation and no
      approval continuation.
- [ ] Implement provider status/role/region/lock-date validation, decimal
      handling, live tax/account/tracking references, and provider-calculated
      total verification.
- [ ] Implement attachment upload/replace, history notes, invoice email, Setup,
      allocations, group membership, status transitions, and all other
      supported action unions.
- [ ] Implement idempotency and exact read-after-timeout reconciliation for
      each mutation family.
- [ ] Add `unimock`, conformance, validation, organisation-binding, app-access,
      duplicate-call, timeout, redaction, and full operation-coverage tests.
- [ ] Run component format, clippy, host tests, Wasm build, and runtime
      direct-write smoke tests.

### Milestone 12: Repository and Runtime Integration

- [ ] Exercise every tool family through the real Wasm component with
      `WasmHostMock`, asserting exact provider traffic, organisation-ref
      resolution, tenant injection, side-effect count, idempotency, output, and
      redaction.
- [ ] Add package tests for identity, scopes, OAuth, organisation routing,
      app-level access, tool schemas, direct-write behavior, absence of
      Xero-specific approval, host allowlists, limits, operation inventory,
      exclusions, assets, and docs.
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
- [ ] Smoke every write action with disposable records, including direct
      execution, duplicate invocation, ambiguous response, refresh,
      organisation removal, and the standard audit record.
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
- Every provider mutation executes through the ordinary app tool path without a
  mandatory human prompt, uses trusted idempotency and reconciliation, and
  produces a standard redacted audit record.
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
