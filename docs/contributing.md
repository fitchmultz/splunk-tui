# Contributing to Splunk TUI

Thank you for contributing.

## Prerequisites

- Rust 1.94.0+
- Make
- Docker (optional, for container and local Splunk workflows)

## IDE Setup

Use the editor and debugger you prefer. Rust Analyzer and CodeLLDB provide a good default experience in VS Code and compatible JetBrains IDEs, but the repository does not depend on tracked editor-specific project files.

## Development Workflow

### First-time setup

```bash
make deps
make build
make ci-fast
```

### Day-to-day loops

Non-mutating checks:

```bash
make format-check
make lint-check
make test
```

Auto-fix loop:

```bash
make fix
make lint-check
make test
```

## Local CI Contract

`make ci-fast` is the canonical fast local gate.

- Non-mutating by default (no formatter/clippy auto-fix)
- Resource-bounded and smoke-focused
- No binary install side effects

`make ci` is the full local gate for release-grade validation.

- Includes full test suite + CI-profile build + docs/examples checks
- Live tests skipped by default for deterministic offline execution (`CI_LIVE_TESTS_MODE=skip`)

Run strict live-test gating when a real Splunk instance is available:

```bash
CI_LIVE_TESTS_MODE=required make ci
```

## Resource-Aware CI and Tests

The Makefile defaults are conservative so CI does not monopolize your machine:

- `CARGO_JOBS=4` (cargo compile parallelism)
- `RUST_TEST_THREADS=1` (Rust test harness concurrency)

Override per run when needed:

```bash
CARGO_JOBS=2 RUST_TEST_THREADS=1 make ci-fast
```

## Docs and Snapshot Workflows

Docs drift check:

```bash
make lint-docs
```

Regenerate docs when expected:

```bash
make generate
```

Fast TUI UX regression loop:

```bash
make tui-smoke
```

Review and accept intentional snapshot changes:

```bash
cargo insta review
```

## Commit and Review Conventions

Use conventional commit style for readability and public history quality:

- `feat: ...`
- `fix: ...`
- `docs: ...`
- `refactor: ...`
- `test: ...`
- `chore: ...`

Before opening a PR:

1. Run `make ci-fast`
2. Verify `git status` is clean
3. Include rationale and verification steps in the PR description

Before publishing/releasing from main:

1. Run `make ci`
2. If live infrastructure is available, run `CI_LIVE_TESTS_MODE=required make ci`

## Security and Secrets

- Never commit `.env` or `.env.test`
- Run `make lint-secrets` before commit
- Install the local pre-commit guard if desired:

```bash
make install-hooks
```

## Optional Build Speedups

These are optional and should not be required for first-time contributors:

- `sccache`: install it locally if you want faster rebuilds; the Makefile detects it automatically and falls back to normal Cargo execution when absent
- `lld`: configure target-specific linker flags in local (untracked) Cargo config overrides
