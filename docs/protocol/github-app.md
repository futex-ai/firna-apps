# GitHub App Protocol

Status: target contract for GitHub package `2.1.0`.

This document defines Firna's GitHub package boundary. The generic platform
owns routing, pinned versions, installation tokens, effect durability,
subscriptions, workstream reconciliation, and delivery. This repository owns
every GitHub header, event, payload, permission, projection, and registration
choice.

Provider references:

- [REST API versions](https://docs.github.com/en/rest/about-the-rest-api/api-versions)
- [GitHub App permissions](https://docs.github.com/en/rest/authentication/permissions-required-for-github-apps)
- [Validating deliveries](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries)
- [Webhook events](https://docs.github.com/en/webhooks/webhook-events-and-payloads)

## Registrations And Rollout

Production uses App ID `4504159`, client ID `Iv23lidBdZ0I2rgwjhXB`, slug
`firna-ai`, setup `https://firna.ai/apps/github/install/setup`, callback
`https://firna.ai/apps/github/install/callback`, and webhook
`https://api.firna.ai/apps/github/webhooks/github_events`.

Stable `br-main` preview uses App ID `4515873`, client ID
`Iv23liSZsLmwSZrxxpzm`, slug `firna-ai-preview`, setup
`https://br-main.preview.firna.ai/apps/github/install/setup`, callback
`https://br-main.preview.firna.ai/apps/github/install/callback`, and webhook
`https://br-main.api.preview.firna.ai/apps/github/webhooks/github_events`.
Ephemeral previews do not install GitHub.

Both registrations and the manifest use exactly this repository permission map:

| Permission | Level |
| --- | --- |
| `actions` | write |
| `administration` | read |
| `checks` | write |
| `statuses` | write |
| `contents` | write |
| `deployments` | write |
| `discussions` | write |
| `issues` | write |
| `merge_queues` | write |
| `metadata` | read |
| `packages` | write |
| `pages` | write |
| `repository_projects` | write |
| `pull_requests` | write |
| `actions_variables` | write |
| `workflows` | write |

Every other repository permission and every organization, account, and
enterprise permission stays at no access. The five current read tools declare
narrow per-tool subsets; the host must not mint them all sixteen grants.

Before `2.1.0` activation, registrations retain the original six configurable
events: `issue_comment`, `issues`, `pull_request`, `pull_request_review`,
`pull_request_review_comment`, and `push`. Firna platform support deploys first,
then this immutable package is published and active installations approve the
new grants. An installation stays on its working old package until live grants
satisfy `2.1.0`; pending approval must not strand its verifier.

Stable preview activates first by selecting the other 38 matrix events and
top-level `installation_target`. Production follows only after preview smoke.
Top-level `meta` and global `security_advisory` remain unselected. Rollback
first deselects the 38 additions and `installation_target`; URLs, secrets, and
approved permissions remain unchanged while the incident is investigated.

## Event Matrix

Published events normalize and remain available for explicit agent
subscription. Acknowledged events authenticate against the installation-pinned
package and return success without normalization, persistence, effects, or
delivery.

| Family | Published | Acknowledged |
| --- | --- | --- |
| Source/repository | `push` | `create`, `delete`, `commit_comment`, `fork`, `gollum`, `release`, `repository`, `repository_dispatch`, `public`, `star`, `watch` |
| Pull requests | `pull_request`, `pull_request_review`, `pull_request_review_comment`, `merge_group` | `pull_request_review_thread`, `merge_queue_entry` |
| Issues/planning | `issues`, `issue_comment` | `issue_dependencies`, `sub_issues`, `label`, `milestone` |
| CI/automation | `check_run`, `check_suite`, `status`, `workflow_job`, `workflow_run` | `workflow_dispatch` |
| Delivery/packages | none | `deployment`, `deployment_status`, `deployment_review`, `deployment_protection_rule`, `registry_package`, `page_build` |
| Discussions | none | `discussion`, `discussion_comment` |
| Repository policy | `branch_protection_configuration`, `branch_protection_rule`, `repository_ruleset`, `security_and_analysis` | `deploy_key`, `exemption_request_push_ruleset` |

This is 16 published and 28 acknowledged definitions. Acknowledged definitions
never appear in public app capability or subscription counts. Unknown event
types still fail closed; there is no wildcard. Security-alert, organization,
enterprise, account, membership-team, marketplace, sponsorship, classic
project, deprecated vulnerability, and provider-ineligible events remain
excluded because their grants or selected-repository consent are absent.

The package also handles controls outside the subscription catalog:

- `ping` returns `200 {"ok":true}`.
- `installation` actions `created`, `unsuspend`, and
  `new_permissions_accepted` reconcile; `deleted` and `suspend` revoke.
- `installation_repositories` actions `added` and `removed` reconcile.
- `installation_target` action `renamed` revision-fences the provider and Firna
  installation labels, then reconciles coverage without becoming agent content.
- mandatory `github_app_authorization` action `revoked` invalidates matching
  user authorization material when present and is an authenticated no-op when
  Firna holds none.

Unrecognized control actions fail closed. Reconciliation re-queries GitHub and
proves the complete grant map; a signed action alone never authorizes Firna.

## Webhook Trust And Bounds

The edge forwards exactly one ordered value for `x-github-delivery`,
`x-github-event`, and `x-hub-signature-256`. Bodies are capped at 262,144 bytes.
The component requires a GUID delivery id, lower-case event identifier, and
`sha256=` followed by 64 lower-case hexadecimal characters. It requests
HMAC-SHA256 with the opaque `webhook_secret` over unchanged UTF-8 bytes and
compares the full digest in constant time before parsing.

Every installation-routed delivery contains a positive installation id and
account id. Published repository events also contain a positive repository id.
Acknowledged events require only that installation identity; repository and
sender metadata are optional and ignored when absent. The app-level
`github_app_authorization` control instead requires the revoked user's positive
sender id. The candidate and pinned verifier return authenticated optional
repository, user, and account-label metadata for the platform actions that use
them. Header/payload disagreement, duplicate headers,
unsigned input, malformed ids, oversized input, and unknown shapes fail closed.
Acknowledged events parse only the common signed envelope; arbitrary nested
provider data is ignored after identity verification and never forwarded.

Published projections contain only bounded identities, actions/states, safe
repository and ref metadata, and canonical `https://github.com/` URLs.
Commit arrays stop at 20 entries; names and titles at 256 characters, messages
at 512, bodies/comments at 2,000, URLs at 2,048, and effect branch hints at
eight. Source snippets, patch text, dispatch payloads, signatures, secrets,
tokens, provider errors, unknown fields, and raw bodies are prohibited from
normalized output, effects, logs, fixtures, and rollout evidence.

## Repository Change Effects

Exactly these 13 published events may emit one version-1
`repository_change`:

| Events | Change kind | Hint behavior |
| --- | --- | --- |
| `push` | `source` | branch from `refs/heads/*`; tag pushes emit none |
| `pull_request` | `pull_request` | head branch, PR number, head SHA |
| `pull_request_review` | `review` | head branch, PR number, head SHA |
| `merge_group` | `merge_queue` | repository scope and head SHA |
| `check_run`, `check_suite`, `status` | `check` | provider-supplied branch hints and head SHA; repository scope when absent, invalid, incomplete, or over eight unique values |
| `workflow_job`, `workflow_run` | `workflow` | provider-supplied branch/PR hints and head SHA; repository scope when absent, invalid, incomplete, or over eight unique values |
| `branch_protection_configuration`, `branch_protection_rule`, `repository_ruleset` | `repository_policy` | repository scope |
| `security_and_analysis` | `security` | repository scope |

`pull_request_review_comment`, `issues`, and `issue_comment` remain published
for subscribers but emit no platform effect. Each effect repeats only the
verified repository id, a closed change/scope enum, bounded unique branches,
optional positive PR number, and optional hexadecimal SHA. Firna always re-reads
the repository's current PR state; effect fields never become snapshot facets.

## Tool And Token Contract

All tools remain `external_read` and emit only `GET` requests. Their token
subsets are:

| Tool | Installation permissions |
| --- | --- |
| `github_list_repositories` | `metadata: read` |
| `github_search_code` | `contents: read`, `metadata: read` |
| `github_read_file` | `contents: read`, `metadata: read` |
| `github_read_pr` | `checks: read`, `contents: read`, `metadata: read`, `pull_requests: read`, `statuses: read` |
| `github_read_issue` | `issues: read`, `metadata: read` |

The component never receives a token. The host resolves
`github_installation`, validates the subset against the pinned flow, requests
that subset from GitHub with no repository selection, and injects the bearer
value into the allowed request. External repository credentials remain narrowed
to one selected repository and the operation's subset.

Input schemas reject unknown properties. Owner/repository values stop at 100
bytes. File paths stop at 1,024 bytes and 16 safe segments; files stop at 256
KiB. Search returns at most 20 rows inside GitHub's first 1,000 results. PR
files and issue comments return at most 10 rows per page. Provider bodies stop
at 1 MiB and serialized success output at 768 KiB.

## Operations And Evidence

Build and validate with:

```sh
cargo build --manifest-path apps/github/component/Cargo.toml \
  --target wasm32-unknown-unknown --locked
firna apps validate apps/github
firna apps package apps/github
cargo test --manifest-path apps/github/tests/platform-runtime/Cargo.toml --locked
```

Pre-activation tests exercise all 44 exact definitions, controls, malformed and
adversarial fixtures, the 16/28 split, effect conformance through the pinned
Firna runtime, duplicate and zero-subscriber acceptance, and a dormant-event
flood beside a published PR event. When a branch-reachable disposable private
repository and test App are available, smoke one PR/review/check path and one
event per family. Retain only delivery ids, event names, permission lists, and
pass/fail results.

Rotate the webhook secret in one maintenance window by changing Firna and the
registration together, then redeliver ping and one published event. Never place
secret values in source, manifests, commands, fixtures, logs, or evidence.
