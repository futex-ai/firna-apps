# GitHub Webhook Events

Status: planned.

Extend the first-party GitHub App with authenticated, installation-scoped
webhook ingestion and bounded event delivery through Firna's existing app
handler-thread architecture. Keep the provider integration read-only and use
the existing Metadata, Contents, Issues, and Pull requests permissions.

The platform-side contract changes belong in `futex-ai/firna`; the package,
component, fixtures, and deployment handoff belong in this repository. The
platform changes must merge and deploy first, after which this repository must
pin the merged platform revision before publishing the updated package.

Provider references:

- [Using webhooks with GitHub Apps](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/using-webhooks-with-github-apps)
- [Validating webhook deliveries](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries)
- [Webhook events and payloads](https://docs.github.com/en/webhooks/webhook-events-and-payloads)

## Confirmed Scope

- Add one app-owned `webhook_secret` and verify GitHub's
  `X-Hub-Signature-256` HMAC-SHA256 over the unmodified request body before
  installation lookup, persistence, acknowledgement, or agent delivery.
- Use `X-GitHub-Delivery` as the provider delivery identifier and
  `X-GitHub-Event` as the provider event type; reject missing, malformed, or
  conflicting trusted headers.
- Route by the verified numeric GitHub installation id and account id, then
  require exactly one matching live Firna workspace installation. Never trust
  repository, account, or installation identifiers from an unsigned payload.
- Handle `installation` and `installation_repositories` as control-plane
  lifecycle events. Suspension, deletion, permission changes, and repository
  selection changes must invalidate affected runtime state immediately.
- Deliver the initial content-event set through the existing
  `app-event-handler` role: `push`, `pull_request`,
  `pull_request_review`, `pull_request_review_comment`, `issues`, and
  `issue_comment`.
- Keep checks, workflows, deployments, security findings, mutations, and
  background repository synchronization out of scope. They require separate
  permissions and product contracts.
- Persist only a typed, bounded, redacted normalized projection. Raw private
  repository payloads, signatures, webhook secrets, tokens, and provider
  errors must not enter audit logs, prompts, or model-visible storage.
- Reuse `POST /apps/{app_id}/webhooks/{ingress_id}` and the generic app event
  queue; do not add a second GitHub-specific ingress stack.
- Reuse generic Settings projections for ingress and event subscriptions; no
  GitHub-specific UI or mockup change is planned.

## Milestone 1: GitHub Webhook And Event Delivery

Summary: implement, deploy, and verify secure GitHub lifecycle and repository
events without increasing the app's read-only provider permissions.

- [ ] Update the platform GitHub App, generic Apps, REST API, deployment, and
      event-handler protocol documents with the exact event allow-list,
      production webhook URL, signature contract, routing identity, normalized
      payloads, acknowledgement behavior, limits, lifecycle transitions,
      redelivery handling, and secret-rotation procedure.
- [ ] Replace installation verification's current empty-event requirement with
      an exact comparison against the approved GitHub App event set while
      retaining the exact read-only permission check.
- [ ] Extend the verified webhook routing contract to carry a typed provider
      installation id in addition to the provider account id, and fail closed
      on unknown, suspended, uninstalled, cross-app, or ambiguous mappings.
- [ ] Add a trait-backed GitHub lifecycle coordinator that invalidates cached
      installation tokens and updates durable installation status for deleted,
      suspended, unsuspended, permission-change, and repository-selection
      deliveries. Mock the trait boundary with `unimock` in unit tests.
- [ ] Bump the GitHub package version and add required `webhook_secret`, HMAC
      capability, `github_events` webhook ingress, the approved event
      subscriptions, `app-event-handler` role, and explicit payload/runtime
      limits to `apps/github/manifest.yaml`.
- [ ] Add component exports for webhook verification, authenticated `ping`
      acknowledgement, and event normalization using the existing app-component
      ABI and host-owned opaque secret handle.
- [ ] Verify only `sha256=` signatures with constant-time comparison over the
      exact raw body. Do not accept SHA-1, decoded/re-serialized JSON, an empty
      secret, unsigned payloads, or a body/header event-type disagreement.
- [ ] Normalize each supported event into a typed bounded projection containing
      only the installation, repository, actor, action, stable object ids,
      canonical URLs, refs/SHAs, and event-specific summary fields needed by
      the handler. Bound or omit commit lists, patches, comments, titles, and
      bodies before persistence.
- [ ] Acknowledge authenticated duplicate deliveries without creating another
      app event, using the GitHub delivery GUID plus app installation identity;
      persist and enqueue a new handler event atomically before returning
      success.
- [ ] Define authenticated no-op acknowledgement for `ping`, unsupported
      implicit GitHub App events, and intentionally ignored activity actions so
      provider redelivery cannot create work or leak payload content.
- [ ] Prove repository content events belong to a repository currently selected
      for the verified installation before handler delivery, including removal,
      transfer, rename, private visibility, and stale-delivery races.
- [ ] Add fixtures and failing-first component/platform tests for valid and
      invalid signatures, altered bodies, missing headers, malformed JSON,
      payload limits, delivery replay, cross-workspace isolation, lifecycle
      transitions, every subscribed event type, ignored events, redaction, and
      unchanged Slack webhook behavior.
- [ ] Update production and preview secret documentation for
      `firna-prod-app-github-webhook-secret` and
      `firna-preview-test-runtime-github-webhook-secret`; keep both values out
      of manifests, source bundles, Terraform state, logs, and chat.
- [ ] Update the GitHub registration runbook to enable **Webhook Active**, set
      the exact deployed HTTPS endpoint, configure a high-entropy secret, and
      select only the approved events after the compatible platform deployment
      is live.
- [ ] Update GitHub package, component, runtime-test, repository, and platform
      READMEs so current behavior, setup, rotation, failure recovery, and the
      read-only event boundary remain consistent.
- [ ] Merge and deploy the platform implementation first; update
      `platform.toml`, the deployment workflow, README command, runtime test
      manifests, and lockfiles to the final merged platform revision together.
- [ ] Run focused component and platform-runtime tests, build the GitHub Wasm
      component, exercise a signed local webhook smoke, and run
      `cargo xtask check` with a 100% pass rate.
- [ ] Use a separately registered non-production GitHub App and test repository
      to smoke `ping`, one lifecycle delivery, one content delivery, one
      duplicate redelivery, and one invalid signature; record only redacted
      evidence.
- [ ] Fetch `origin/main`, audit additions and deletions, run `git add -A`,
      commit all implementation and documentation with Conventional Commits,
      and push the current branches without renaming them.
- [ ] After each repository's verified commit is pushed, run
      `cargo xtask review`, record every finding with severity, context,
      impact, lettered solution options, and a recommendation, and let the user
      decide which findings to implement.

Exit criteria: Firna validates GitHub deliveries with the configured app
webhook secret, safely applies installation lifecycle changes, and produces at
most one bounded workspace event for the exact installation from each
supported repository delivery. All checks pass, and both repositories are
committed, pushed, reviewed, and deployed in dependency order.
