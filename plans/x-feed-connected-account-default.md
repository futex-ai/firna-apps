# X Feed Connected-Account Default

- Status: Active
- Target branch: `origin/main`
- Last updated: 2026-08-14

## Outcome

Make `x_get_user_feed` target the explicitly selected X connection when
`user_id` is omitted. Preserve explicit user ids, validate the resolved account
before using it in a provider URL, report the extra lookup at its real cost,
and keep every path bounded.

## Milestone 1: Implement the Connected-Account Fallback

At the end of this milestone omitted user ids resolve the authenticated account
and explicit user ids keep their direct request path.

- [x] Add a regression test that reproduces the rejected home-feed call.
- [x] Resolve an omitted id through one authenticated `/2/users/me` request.
- [x] Reject malformed provider account ids as a provider contract failure.
- [x] Include the identity lookup in successful User-read usage.
- [x] Keep local input validation ahead of provider dispatch.

## Milestone 2: Align the Package Contract

At the end of this milestone the schema, immutable price cap, public docs, and
real Wasm behavior describe one contract.

- [x] Bump the package and component versions for the changed price contract.
- [x] Explain the connected-account default in the model-visible tool schema.
- [x] Raise the feed User-read maximum from 25 to 26.
- [x] Update the X protocol and package/component READMEs.
- [x] Add a runtime smoke test for the two-request omitted-id path.

## Milestone 3: Run the Complete Repository Gate

- [x] Run component formatting, clippy, native tests, and the locked Wasm build.
- [x] Run the X platform-runtime test suite and package validation smokes.
- [x] Run `cargo xtask check` and require a 100% pass rate.
- [x] Inspect `git diff --check`, the complete diff against `origin/main`, and
  all deletions.

## Milestone 4: Commit and Push the Checked Work

- [x] Run `git add -A` so every source, test, manifest, protocol, README, lock,
  and plan change is tracked.
- [x] Commit with a Conventional Commit message.
- [x] Push the current branch without renaming it.
- [x] Recheck the committed diff and deletion list against `origin/main`.

## Milestone 5: Run Post-Push Review

Current blocker: the sandbox `codex` CLI is not authenticated. Two post-push
`cargo xtask review` attempts reached the reviewer but returned HTTP 401 before
producing findings. Rerun after reviewer authentication is available.

- [ ] Run `cargo xtask review` against `origin/main` after the push.
- [ ] Investigate every finding without automatically changing reviewed code.
- [ ] Report numbered findings with severity, context, impact, lettered solution
  options, and a recommended option.
- [ ] Evaluate whether broader tests, rules, abstractions, or architecture would
  prevent each finding class more effectively than a direct patch.
