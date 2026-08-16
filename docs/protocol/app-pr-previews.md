# Static App Pull-Request Preview

- Status: proposed 2026-08-16; repository-side implementation prepared here
- Related protocols: [app deployment](app-deployment.md) and
  [preview events](app-pr-preview-events.md)

Pull requests in `futex-ai/firna-apps` may claim one fixed review slot by
adding the `preview` label. The slot uses a known-good Firna platform release
and app packages from one exact pull-request head SHA. It tests app changes
without changing production, `br-main`, or ordinary Firna `pr-N` previews.

The first rollout has one non-preemptible slot, rejects forks and Firna PR
code, and preserves no catalog, installation, user, artifact, or database
state between app candidates. Provider registrations and Secret Manager
values are durable. Environment identity appears in GitHub developer feedback,
not in product views.

## Environment Contract

| Environment | Firna revision | App revision | Purpose |
| --- | --- | --- | --- |
| production | released | deployed `main` | live product |
| `br-main` | validated Firna `main` | `firna-apps@main` | stable integration |
| `br-apps` | last validated Firna `main` | exact app PR SHA | app review |
| `pr-N` | exact Firna PR SHA | `firna-apps@main` | platform review |

`br-apps` has fixed identity and disposable state:

- deployment slug: `br-apps`
- namespace: `firna-preview-br-apps`
- product URL: `https://br-apps.preview.firna.ai`
- API URL: `https://br-apps.api.preview.firna.ai`
- Platform API host: `br-apps-platform.api.preview.firna.ai`
- hosted Git host: `br-apps.git.preview.firna.ai`
- app class/prefix: `review` / `review-app`
- label: `preview`

Existing wildcard preview DNS and certificates must cover every host before
enablement. Every accepted deploy drains active sandboxes, deletes the
namespace and disposable PVCs, recreates the environment, and then submits all
targeted candidate packages. Resetting is mandatory because approved
`(app_id, version)` records are immutable while two unmerged revisions may use
the same version for different source.

The root [`deploy.toml`](../../deploy.toml) declares `br-apps` with
`automatic = false`. That value is required and excludes it from scheduled,
`main`, repository-dispatch, manual, and all-instance matrices in
`deploy-apps.yml`. Only the dedicated platform receiver may select it.
Production and `br-main` are explicitly or implicitly automatic.

`review` is a closed deployment class mapped to `review-app`. A missing
per-app `deploy.toml` targets it, as it does every class. An explicit class
list must name `review`; this is appropriate only after its test-provider
registration and every required `review-app-*` value exist. The metadata-only
merge gate creates missing containers and checks enabled-version metadata but
never reads values.

## Trusted Request Controller

[`app-preview-request.yml`](../../.github/workflows/app-preview-request.yml)
runs only default-branch controller code. It has two metadata triggers:

- successful completion of canonical `CI`, for label-before-CI ordering; and
- `pull_request_target` on `labeled`, `unlabeled`, `closed`, `reopened`, and
  `edited`, for CI-before-label ordering, retargeting, and release.

The controller checks out `main` with persisted credentials disabled. It never
checks out a merge ref, executes candidate commands, reads candidate files, or
places pull-request-controlled text into a shell command. It obtains identity
from the signed event and reloads current PR, label, permission, and CI
metadata through GitHub's API.

A deploy is eligible only when:

1. the PR and head repository are `futex-ai/firna-apps`;
2. the PR is open against `main` and still has `preview`;
3. the event/request SHA equals its current 40-character head SHA;
4. canonical `CI` succeeded for that SHA; and
5. the active label was applied by an actor whose current permission is
   `write`, `maintain`, or `admin`.

Forks fail closed. Branch names are never deployment identities. The
controller neither searches for the newest labelled PR nor sends a mutable
ref. Removing `preview`, closing a same-repository PR, or retargeting it away
from `main` sends a release hint carrying that PR's exact identity. The
workflow intentionally has no target-branch event filter so those cleanup
events still run after a retarget; deploy eligibility continues to require
`main` in trusted code.

`FIRNA_PLATFORM_PREVIEW_DISPATCH_TOKEN` is a fine-grained repository secret
restricted to Contents write on `futex-ai/firna`, which is the permission
needed for repository dispatch. It is distinct from the read-only
`FIRNA_REPOSITORY_TOKEN` and every deployment credential.

The strict request and result JSON contracts are defined in
[preview events](app-pr-preview-events.md) and enforced by
`scripts/app_preview_contract.py`. Requests are hints, never authorization;
the platform independently reloads GitHub state.

## Platform Receiver and Lease

All platform receiver actions use one non-cancelling concurrency group. The
authoritative lease lives in a durable control namespace outside the candidate
namespace and records source repository, PR number, app SHA, platform SHA, and
correlation id. Namespace annotations are diagnostic mirrors only.

- A free slot may be claimed by an eligible request.
- Its owner may replace it with a newer current SHA after a full reset.
- Another PR receives `busy` and cannot preempt it.
- Owner unlabel/close deletes and releases it.
- Every cleanup compares repository and PR number before deletion.
- Scheduled reconciliation releases an owner that is closed, unlabelled,
  retargeted, or otherwise no longer has the canonical source identity.

To transfer the slot, remove `preview`, wait for `released`, and add it to the
next PR. A stale deploy becomes `superseded`; a stale release cannot remove a
different owner's slot.

`br-apps` uses one immutable manifest from the last successful, fully smoke-
tested `br-main` rollout. That manifest pins the platform SHA and image digests
for server, worker, app builder, Platform API, and the review web build. A
failed or rolled-back platform rollout never updates it, and one candidate may
not combine artifacts from different releases.

After acquiring the lease, the platform receiver:

1. records one platform release manifest;
2. drains the old revision and recreates disposable state;
3. deploys one preview replica per service from that manifest;
4. checks out `futex-ai/firna-apps` at the validated SHA into a data directory
   and disables credentials after checkout;
5. uses only its pinned CLI and trusted platform scripts;
6. discovers every app targeting `review`, not merely changed files;
7. reads only `review-app-*` values through the review identity;
8. verifies API/web health and exact catalog versions; and
9. returns one terminal result.

Candidate scripts are never invoked. App source is input to the normal
isolated app-builder boundary. The platform remains app-agnostic: ids, secret
names, targeting, and provider configuration come from the exact checkout.

The namespace, database, Redis, and `br-apps/` artifact prefix are isolated
from every other environment. The artifact prefix has age-based deletion.
No bootstrap password, provider credential, storage prefix, or deployment
identity is shared with production or `br-main`.

## Feedback and Security

[`app-preview-result.yml`](../../.github/workflows/app-preview-result.yml)
accepts only the platform result dispatch. The default-branch handler rejects
unknown fields and values, reloads the PR, and ignores stale identity or
correlation. A lease-matching release may update a closed, unlabelled, or
retargeted PR after its head advances, but is ignored if that PR is currently
eligible again.

For a current result it maintains one marker-delimited bot comment and one
advisory `app preview` check on the exact app SHA. Ready feedback names both
immutable SHAs and links to the fixed product/API URLs and platform run.
Other statuses never expose environment URLs. Making the check required is a
separate repository-policy decision.

The review identity may read only `review-app-*` values and the
`br-apps` bootstrap secret, and may mutate only review resources. Values are
masked before export and never enter source, manifests, archives, comments,
artifacts, dispatches, or logs. The app-secrets project IAM condition is
managed in [`infra/gcp/apps`](../../infra/gcp/apps/README.md).

## Verification and Rollout

Repository checks cover class/config/matrix rules, implicit and explicit app
selection, metadata-only secret discovery, strict event parsing, both CI/label
orders, latest-CI and active-label resolution, stale/ineligible/fork PRs,
same-owner SHA updates, retarget and release races, feedback correlation,
workflow assertions, and actionlint.

Rollout must land and verify the Firna manifest, receiver, singleton lease,
review identity, result callback credential, and cleanup first. Then operators
provision provider registrations and every `review-app-*` value before merging
this repository's targeting and workflows.

The end-to-end smoke uses real same-repository PRs and verifies initial claim,
a second push, exact catalog versions, fixed-origin access, one provider
callback, a competing-PR `busy` result, label-removal cleanup, and no
production/`br-main` mutation. Rollback disables the request workflow and
releases `br-apps`; it does not change other environments or app manifests.
