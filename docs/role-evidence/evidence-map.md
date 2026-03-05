# Evidence Map

## Purpose

This map links release-readiness claims to reproducible evidence in the repository.

## Reliability and Correctness

Primary evidence:

- `make ci-fast` ✅
- `make ci` ✅
- `CI_LIVE_TESTS_MODE=required make ci` ✅

Supporting evidence:

- `docs/role-evidence/verification-receipts.md`
- `docs/role-evidence/qualitative-dogfood-2026-03-05.md`

## Safety and Security

- Secret-commit guard: `scripts/check-secrets.sh` (wired into `make lint-secrets`, `ci-fast`, `ci`)
- Help-output env value redaction for CLI/TUI env-backed flags
- Security policy: `SECURITY.md`

## UX/DX Confidence

- Resize robustness evidence: `crates/tui/tests/resize_tests.rs` + qualitative tmux resize stress run
- Search ergonomics evidence: query normalization in `crates/cli/src/commands/search.rs` + `crates/cli/tests/search_tests.rs`
- Onboarding/secret trust evidence: help redaction tests in CLI/TUI test suites

## Maintainability and Architecture

- Architecture narrative: `docs/architecture.md`
- CI strategy and resource controls: `docs/ci.md`
- Reviewer checklist: `docs/reviewer-verification.md`
- Release status: `docs/release-readiness.md`

## Developer Productivity

- Fast gate: `make ci-fast`
- Full gate: `make ci`
- Local-first quality contract: `Makefile`

## Remaining Risk

- Strict live validation depends on access to a reachable Splunk environment and valid credentials.
- Long-tail UX paths should continue to be expanded via periodic qualitative dogfood runs.
