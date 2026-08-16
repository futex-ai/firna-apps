# GitHub App Protocol

Status: implemented by GitHub package `2.0.4`.

This protocol defines the Firna-owned GitHub package boundary. The platform
owns installation routing, provider-installation records, token minting,
subscription eligibility, durable deduplication, lifecycle transactions, and
delivery. The package owns tool request construction, provider response
projection, signature verification, lifecycle classification, and event
normalization.

Provider references:

- [REST API versions](https://docs.github.com/en/rest/about-the-rest-api/api-versions)
- [GitHub App permissions](https://docs.github.com/en/rest/authentication/permissions-required-for-github-apps)
- [Validating webhook deliveries](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries)
- [Webhook events and payloads](https://docs.github.com/en/webhooks/webhook-events-and-payloads)

## Registration And Authentication

Production has App ID `4504159`, client ID `Iv23lidBdZ0I2rgwjhXB`, and slug
`firna-ai`. Setup returns to `https://firna.ai/apps/github/install/setup`;
callback completion uses `https://firna.ai/apps/github/install/callback`.

The stable `br-main` preview has App ID `4515873`, client ID
`Iv23liSZsLmwSZrxxpzm`, and slug `firna-ai-preview`. Setup returns to
`https://br-main.preview.firna.ai/apps/github/install/setup`; callback
completion uses
`https://br-main.preview.firna.ai/apps/github/install/callback`. Ephemeral
`pr-N` previews do not install GitHub because their URLs are not registered.

The static `br-apps` review slot uses a third test registration. Its setup and
callback return to `https://br-apps.preview.firna.ai/apps/github/install/setup`
and `https://br-apps.preview.firna.ai/apps/github/install/callback`; its webhook
targets `https://br-apps.api.preview.firna.ai/apps/github/webhooks/github_events`.
Registration identifiers and private values are supplied only through
`review-app-github-*` secrets.

The manifest retains Contents write and Pull requests write for the external
repository workflow. It adds Issues read and retains Metadata read. The five
component tools are independently declared `external_read` and emit only
GitHub API GET requests.

A workspace may connect multiple GitHub-side installations. Tool calls carry
the Firna installation id to the trusted host. The host resolves an
`installation_token` credential and injects it as bearer authorization; the
component never receives or serializes the token.

The manifest resolves `app_slug`, `callback_url`, `client_id`, and `setup_url`
from deployment-owned app values, allowing one immutable package to select the
correct registration per environment. It also declares `client_secret`,
`private_key`, and `webhook_secret`. Values must not appear in manifests,
packages, logs, fixtures, prompts, command arguments, or Wasm artifacts.

## Tool Contract

Every input schema rejects unknown properties. Owners and repositories are
bounded to 100 bytes and validated before URL encoding. Page and number values
are positive signed 32-bit integers.

### `github_list_repositories`

Inputs are optional `page` and `per_page`; defaults are 1 and 30, and the
maximum page size is 50. The component calls
`GET /installation/repositories` and returns a bounded repository projection,
current page, and a validated numeric `next_page` when present.

### `github_search_code`

`query` is required, trimmed, non-blank, and at most 256 Unicode scalars.
`owner` and `repository` must be supplied together or both omitted.
`language`, `path`, `page`, and `per_page` are optional; page size
defaults to and is capped at 20.

The query is quoted as one literal before qualifiers are added. Quotes and
backslashes are escaped; provider query encoding remains a host concern.
Search is limited to GitHub's first 1,000 results. Output contains bounded code
matches, at most five text-match fragments per row, result metadata, and a
numeric next page inside that window.

### `github_read_file`

`owner`, `repository`, and `path` are required; `ref` is optional.
Paths are relative, at most 1,024 bytes and 16 non-empty segments, contain no
control characters, and contain no `.` or `..` segment.

The component resolves the ref to a commit, walks one non-recursive Git tree per
path segment, and accepts only regular or executable blobs. It rejects
directories, symlinks, and submodules before calling Contents with the resolved
commit SHA. The returned path and blob SHA must match. Content must be valid
base64 for an exact UTF-8 file of at most 256 KiB, with no unsupported control
characters.

### `github_read_pr`

`owner`, `repository`, and positive `number` are required.
`include_files` defaults true. `files_page` defaults 1 and
`files_per_page` defaults to and is capped at 10. Changed-file pagination may
not cross GitHub's 3,000-file window.

Output contains typed pull request details and, when requested, one bounded
changed-file page. Patch previews are capped at 8,192 bytes and can be removed
from later rows to keep the serialized output within budget.

### `github_read_issue`

`owner`, `repository`, and positive `number` are required.
`include_comments` defaults true. `comments_page` defaults 1 and
`comments_per_page` defaults to and is capped at 10.

A provider object containing the pull-request discriminator returns
`use_github_read_pr`. Issue bodies are previewed at 65,536 bytes and comment
bodies at 8,192 bytes. Labels and assignees are bounded and report truncation.

## Provider And Error Contract

All component requests target `https://api.github.com`, carry
`Accept`, `User-Agent`, and `X-GitHub-Api-Version: 2026-03-10`, request a
60-second timeout, and cap provider bodies at 1 MiB. The component's own
serialized success budget is 768 KiB.

Host credential absence returns `auth_required` for
`github_installation`. Invalid caller input and inaccessible or missing
resources return stable invalid-request errors. Provider rate limiting can
return a bounded retry delay. Access denial, provider unavailability,
contract-invalid responses, and oversized responses are distinct stable
errors. Provider bodies, transport details, credential failures, and secrets
must not be copied into errors.

## Webhook Trust Contract

The public endpoint is `POST /apps/github/webhooks/github_events`. Production
registers `https://api.firna.ai/apps/github/webhooks/github_events`; stable
preview registers
`https://br-main.api.preview.firna.ai/apps/github/webhooks/github_events`.

The edge forwards only these ordered, duplicate-preserving raw headers:

- `x-github-delivery`
- `x-github-event`
- `x-hub-signature-256`

The body limit is 262,144 bytes. Verification requires exactly one value for
each header, a GUID-shaped delivery id, a lower-case provider event identifier,
and a signature of `sha256=` plus 64 lower-case hexadecimal characters. The
component asks the host to calculate HMAC-SHA256 with the opaque
`webhook_secret` over the unchanged UTF-8 body and compares the complete
signature in constant time before parsing or routing the payload.

The signed payload must contain a positive numeric GitHub installation id and
account id for every non-ping delivery. Content events additionally require a
positive repository id and event-specific typed objects. A header/payload shape
disagreement fails closed.

The six subscribable events are `push`, `pull_request`,
`pull_request_review`, `pull_request_review_comment`, `issues`, and
`issue_comment`. Agents subscribe explicitly; installation alone produces no
subscription.

Authenticated `ping` returns HTTP 200 with `{"ok":true}` and creates no
event. GitHub's implicit lifecycle events classify as follows:

- installation `created`, `unsuspend`, or
  `new_permissions_accepted`: reconcile;
- installation `deleted` or `suspend`: revoke;
- installation-repositories `added` or `removed`: reconcile.

Reconcile and revoke invalidate cached installation tokens. The platform owns
the durable provider-installation transition and repository coverage observer.
Unsupported event types and unrecognized lifecycle actions are rejected.

## Normalized Event Contract

Normalization runs only for a declared content event after authoritative
installation-pinned verification. Output repeats the trusted app,
installation, provider account, delivery, and event identities. Source metadata
contains the positive installation, repository, and actor ids plus bounded
repository and actor names when present.

Payloads contain a typed repository, actor, action, and event-specific
projection. Commit lists stop at 20 entries. Titles and names stop at 256
characters, commit messages at 512, and bodies or comments at 2,000. URLs are
retained only when they begin with `https://github.com/` and are at most 2,048
characters.

Unknown payload fields, webhook signatures, tokens, secrets, provider errors,
and patch text are never normalized. The platform deduplicates accepted
deliveries by Firna installation and GitHub delivery id and delivers at most one
event per eligible subscription.

## Operations

Each GitHub App registration must enable its environment's webhook URL,
configure the same high-entropy secret as that environment's
`webhook_secret`, and select only the six subscribable events. GitHub supplies
installation and installation-repositories events implicitly.

Rotate the webhook secret in a maintenance window because GitHub and Firna each
use one active value. Update both sides, redeliver authenticated ping and one
content event, verify duplicate handling, and disable the previous secret
version. Rollback restores both prior values together.
