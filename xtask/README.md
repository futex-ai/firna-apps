# xtask

`xtask` is the developer automation binary for the Firna-owned app repository.
Use it when validating packages locally or running the same checks as CI.

## Responsibilities

- Run repository, workflow, Rust formatting, lint, build, and test checks.
- Use the complete component manifest inventory for every standalone component
  format, lint, build, and unit-test command.
- Keep every standalone platform-runtime test in the check plan.
- Launch a read-only Codex review of the branch diff against `origin/main`.

## What This Crate Does

The binary orchestrates existing tools and stops on the first failed command.
App logic remains in `apps/`; repository-specific structural checks remain in
`scripts/repository_audit.py`.

## Quick Start

```sh
cargo xtask check
cargo xtask repository-audit
cargo xtask review
```

## Development

Run the root crate checks after changing the command plan:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

### Key Code

- `src/check.rs` defines the complete verification command plan.
- `src/command.rs` owns subprocess execution.
- `src/review.rs` defines the post-push review invocation.

### Related Docs

- [Repository development guide](../README.md#development)
- [App package guide](../apps/README.md)
