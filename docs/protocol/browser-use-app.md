# Browser Use App Protocol

Status: planned by [`plans/browser-use-app.md`](../../plans/browser-use-app.md).
The package must not be released until the platform prerequisite below is
merged and this repository pins that merged revision.

## Purpose

The first-party Browser Use app lets an explicitly consenting workspace start,
inspect, and cancel one-off Browser Use Cloud agent runs. Browser Use executes
each natural-language goal in a managed browser and returns the result
asynchronously. Firna prepays Browser Use and charges the authorizing
workspace's Firna credit wallet for the exact final provider-reported run cost,
bounded by the immutable app-version cap.

V1 uses Browser Use Cloud API V4, the provider's recommended API for new
long-horizon integrations. It does not expose raw cloud browsers, persistent
profiles, reusable sessions, follow-up turns, files, recordings, live-view
URLs, provider integrations, schedules, user-supplied secrets, or workspace
API keys.

## Package and Provider Boundary

The package contract is:

- directory and manifest id: `apps/browser-use` and `browser-use`
- catalog name and initial version: `Browser Use` and `1.0.0`
- source kind and install policy: `built_in` and `explicit`
- connection mode: `single`
- provider origin: `https://api.browser-use.com/api/v4`
- allowed provider host and methods: `api.browser-use.com`, `GET`, and `POST`
- app-owned secrets: `api_key` and `task_handle_key`
- injected credential header: `X-Browser-Use-API-Key`
- crypto capability: HMAC-SHA256 for installation-bound task handles
- ingress: none

Production and stable preview use separate Browser Use API keys, credit pools,
and HMAC keys. Values are supplied through the repository-owned app-secret
deployment path and never enter manifests, bundles, logs, task prompts,
component output, or chat. The provider API key is injected by the trusted HTTP
host; component code handles only an opaque credential reference.
`task_handle_key` is not rotated while an open task exists. Emergency rotation
first disables new starts, reconciles or cancels every open run through the
provider account, and only then replaces the key; the provider API key may
rotate independently because it is not part of handle identity.

Each run uses model `gpt-5.6-luna`, `maxCostUsd: "1.000000"`, a US managed
proxy, and recording disabled. The component omits provider workspaces,
profiles, existing sessions, attachments, model parameters, and judging, so a
run is isolated and one-off. The model and request contract must be rechecked
against the V4 OpenAPI document before release; a provider contract change
requires a new app version rather than a silent production fallback.

## Platform Prerequisite

The platform pricing schema must add an optional `reports_tasks` list to a
`pricing.tools` entry. It declares task ids whose terminal usage that tool may
report without opening a new task or carrying its own price. The Browser Use
manifest uses:

```yaml
pricing:
  tools:
    - tool: browser_use_start_task
      opens_task: browser_use_run
    - tool: browser_use_get_task
      reports_tasks: [browser_use_run]
    - tool: browser_use_cancel_task
      reports_tasks: [browser_use_run]
  tasks:
    - task: browser_use_run
      kind: usage_reported
      max_cost_usd_micros_per_task: 1000000
      max_task_duration_seconds: 3600
      usage_reporter_export: report-task-usage
```

A reports-only call takes no new wallet hold and creates no per-call charge.
Its component response uses the typed app-result envelope so the trusted host
can validate terminal events. The host accepts an event only for a task listed
by that exact tool, resolves it against the calling workspace, app,
installation, and provider task id, and settles it idempotently against the
opening task's pinned version. It rejects task-open events, undeclared task
ids, cross-installation ids, malformed usage, and duplicate conflicting
reports. Disabling new app charging must not prevent an already-open task from
being reconciled.

## Task Handles and Ownership

`browser_use_start_task` returns an opaque task handle, not a provider session,
workspace, event, or live-view URL. A V1 handle contains a version, the provider
run UUID, and an HMAC over the version, current Firna installation UUID, and run
UUID using `task_handle_key`. Status and cancel calls recompute the HMAC and
compare it in constant time before contacting Browser Use.

A handle created by another installation, a malformed handle, an unsupported
version, or an invalid signature is rejected as `invalid_task_handle`. Handles
are capability references, not secrets suitable for authentication outside
this app. Provider run ids still appear only inside the signed handle and the
host-only task lifecycle event.

Browser Use V4 currently documents neither an idempotency key nor a client
reference on run creation. The component sends one create request and never
retries it automatically. If dispatch succeeds but the response is lost, the
call returns `task_outcome_unknown`; the task reserve remains fail-closed and
the usage reporter must not claim the task is absent. After bounded reporter
failures, the platform moves the hold to `reconciliation_required` for an
operator to reconcile against Browser Use records. A newly documented provider
idempotency or client-reference facility should replace this manual exception
in a later protocol version.

## Tools

### `browser_use_start_task`

Input is one `task` string containing 1 through 8,000 Unicode scalar values.
The task must describe the complete one-off goal. The component sends exactly
one `POST /runs`, validates the V4 create response, and returns:

```json
{"task_handle":"<opaque>","status":"queued"}
```

The status is the provider's closed `queued`, `dispatching`, `running`,
`completed`, `failed`, or `cancelled` value. A valid create response emits one
host-only `task_opened` event for `browser_use_run` and the run UUID. If create
already reports a terminal status, the run remains opened because the create
response does not contain final cost; a later get, cancel, or reporter call
settles it.

This tool is classified `external_write`: a broad browser goal can submit
forms or otherwise change an external site. Firna does not infer that a
natural-language goal is read-only.

### `browser_use_get_task`

Input is one signed `task_handle` of at most 256 bytes. The component first
sends `GET /runs/{id}/status`. For `queued`, `dispatching`, or `running`, it
returns only the handle and status. For a terminal status, it fetches
`GET /runs/{id}` once and returns the handle, status, and:

- `result` for `completed` when the provider supplies it, bounded to 20,000
  Unicode scalar values;
- `result_truncated: true` when a longer result was shortened; or
- stable `failure: task_failed` or `failure: task_cancelled` without the raw
  provider error.

Every validated terminal summary emits `task_terminal` with the exact run UUID
and final usage. Repeated terminal reads may repeat that event; host settlement
is idempotent and never charges twice.

### `browser_use_cancel_task`

Input uses the same handle contract. The component sends exactly one
`POST /runs/{id}/cancel`, which Browser Use documents as idempotent and as
stopping further model billing. It validates the returned terminal summary,
returns the same bounded terminal shape as get, and emits the same final usage.
It never reports a zero-cost cancellation merely because the result failed or
was cancelled: already-consumed provider work remains billable.

## Usage and Settlement

Starting a run reserves $1.00 (1,000,000 micro-USD) from the workspace wallet
before provider dispatch. The provider receives the same $1.00 hard limit.
Get and cancel calls are free; they only observe or terminate the already-held
task. A workspace without enough spendable credit, or a deployment where paid
app calls are unavailable, fails before Browser Use receives a request.

The terminal `totalCostUsd` string is parsed as a non-negative decimal without
binary floating point and rounded upward to the next micro-USD when it has more
than six fractional digits. This can add less than one micro-USD and prevents
systematic under-recovery. Completed, failed, and cancelled provider runs all
report their consumed cost. A zero provider cost is a valid zero settlement.
The host clamps an over-cap report to $1.00 and audit-flags it; the Browser Use
account budget remains the backstop for provider defects or price changes.

The `report-task-usage` export accepts only the pinned platform request. For a
provider-task lookup it performs the same status-then-summary sequence and
returns `running` or exact terminal reported cost. Because V4 cannot currently
look up a run by Firna opening operation id, that lookup fails closed rather
than returning `absent`. Missing, malformed, negative, non-finite, or otherwise
untrusted cost data never releases or settles a hold automatically.

No usage or financial value is included in agent-visible output. Owners and
admins see the immutable maximum, open reservation, and settled charge through
the platform's existing app, Usage, and Billing surfaces.

## Errors, Privacy, and Limits

Handled errors are `invalid_request`, `invalid_task_handle`, `task_not_found`,
`rate_limited`, `provider_budget_exhausted`, `provider_unavailable`,
`provider_contract_error`, and `task_outcome_unknown`. They contain stable
public-safe copy and optional bounded retry delay only. Raw provider bodies,
task prompts echoed by the provider, internal Browser Use ids outside the
signed handle, API keys, HMAC material, live URLs, token counts, cost values,
and billing identifiers are never exposed.

Local schema failures become `invalid_request`. A provider 404 on get or
cancel becomes `task_not_found`; 400, 409, and 422 become `invalid_request`
with reason `provider_rejected_request`; 402 becomes
`provider_budget_exhausted`; 429 becomes `rate_limited`; and 401, 403, unknown
4xx responses, and non-create transport/5xx failures become redacted
`provider_unavailable`. A create timeout,
transport loss, missing status, 5xx response, or malformed/truncated 2xx body
is always `task_outcome_unknown` because the provider may have opened a run.
No provider request is retried automatically.

Provider requests time out after 30 seconds and accept at most 262,144 response
bytes. Oversized, truncated, malformed, or schema-incompatible responses become
`provider_contract_error`, except an ambiguous create becomes
`task_outcome_unknown`. Status polling is explicit; the component does not
sleep, loop, stream events, paginate project history, or poll in the
background. The platform's task sweep is the recovery path after the one-hour
deadline.

V1 sends no credentials or Firna attachments into the browser, persists no
provider profile or session, and returns no screenshots, recordings, files, or
browser URLs. A live smoke test must use a public, non-destructive goal and
retain only redacted task status and aggregate cost evidence.

Provider behavior and prices were checked on 2026-08-11 against [V4 API],
[create run], [get run], [get run status], [cancel run], and [pricing]. The
Browser Use account billing page is authoritative before credit purchase or
production launch.

[V4 API]: https://docs.browser-use.com/cloud/api-v4-overview
[create run]: https://docs.browser-use.com/cloud/api-v4/runs/create-run
[get run]: https://docs.browser-use.com/cloud/api-v4/runs/get-run
[get run status]: https://docs.browser-use.com/cloud/api-v4/runs/get-run-status
[cancel run]: https://docs.browser-use.com/cloud/api-v4/runs/cancel-run
[pricing]: https://browser-use.com/pricing.md
