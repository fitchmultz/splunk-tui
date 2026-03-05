# Evidence Map

## Purpose

This map links release-readiness claims to reproducible evidence in the repository.

## Reliability and Correctness

- Full local gate passes: `make ci`
- PR-equivalent gate passes: `make ci-fast`
- Deterministic secret guard: `make lint-secrets`

### Latest verification run

- Date: **2026-03-05**
- Commands run successfully:
  - `make lint-secrets`
  - `make ci-fast`
  - `make ci`

## Safety and Security

- Secret-commit guard: `scripts/check-secrets.sh` (wired into `make lint-secrets`, `ci-fast`, `ci`)
- Security policy: `SECURITY.md`
- History-cutover safety runbook: `docs/public-release-runbook.md`

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

- Public history cutover execution is still required before visibility flip (see `docs/public-release-runbook.md`).
