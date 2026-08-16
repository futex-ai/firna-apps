# Static App Preview Events

- Status: proposed 2026-08-16
- Parent protocol: [static app pull-request preview](app-pr-previews.md)

Both repository dispatches use closed version-one JSON objects. Unknown or
missing fields, unknown enum values, booleans in integer fields, non-positive
PR numbers, and SHAs other than 40 lower-case hexadecimal characters are
invalid. The source repository must be exactly `futex-ai/firna-apps`.

## Request

The app repository sends event type `firna-app-preview-request` to
`futex-ai/firna`:

```json
{
  "schema_version": 1,
  "action": "deploy",
  "source_repository": "futex-ai/firna-apps",
  "pr_number": 123,
  "head_sha": "0123456789abcdef0123456789abcdef01234567",
  "correlation_id": "futex-ai/firna-apps#123@0123456789abcdef0123456789abcdef01234567"
}
```

`action` is `deploy | release`. `correlation_id` is derived exactly as
`<source_repository>#<pr_number>@<head_sha>`. A release uses the same shape
and the latest SHA known to its signed PR event.

The receiver rejects an invalid request before reading or mutating its lease.
It then independently reloads the current PR, label, label actor permission,
head SHA, base, source repository, and canonical CI result. A stale deploy is
reported as `superseded`; release compares source repository and PR number
with the lease before cleanup.

## Result

The platform sends event type `firna-app-preview-result` to this repository:

```json
{
  "schema_version": 1,
  "status": "ready",
  "source_repository": "futex-ai/firna-apps",
  "pr_number": 123,
  "head_sha": "0123456789abcdef0123456789abcdef01234567",
  "correlation_id": "futex-ai/firna-apps#123@0123456789abcdef0123456789abcdef01234567",
  "platform_sha": "89abcdef0123456789abcdef0123456789abcdef",
  "product_url": "https://br-apps.preview.firna.ai",
  "api_url": "https://br-apps.api.preview.firna.ai",
  "run_url": "https://github.com/futex-ai/firna/actions/runs/123456789",
  "failure_code": null,
  "owner_pr_number": 123
}
```

`status` is `ready | failed | busy | released | superseded`. `run_url` is
always the HTTPS URL of a `futex-ai/firna` Actions run. `platform_sha` is a
40-character SHA or null when the receiver could not select a release; it is
required for `ready`.

Status invariants are closed:

| Status | URLs | Failure code | Owner |
| --- | --- | --- | --- |
| `ready` | fixed product and API URLs | null | request PR |
| `busy` | both null | null | positive PR different from request PR |
| `failed` | both null | required | null |
| `released` | both null | null | null |
| `superseded` | both null | null | null |

Failure codes are `invalid_request`, `stale_request`, `ci_not_green`,
`slot_busy`, `platform_release_unavailable`, `environment_deploy_failed`,
`app_submission_failed`, and `smoke_failed`.

The result sender uses a platform-held token restricted to this repository.
The handler checks the current PR source, number, base, label, and SHA before
feedback. A `released` result may update its matching closed, unlabelled, or
retargeted PR even after the head advances, but cannot overwrite feedback once
the PR is eligible again. All other ineligible or stale results are ignored
without mutation.
