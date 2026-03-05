# 5–10 Minute Demo Script

## Goal

Demonstrate that the repo is production-minded, deterministic, and easy to validate.

## 1) Clone + quick orientation (1 minute)

```bash
git clone <repo-url>
cd splunk-tui
make help
```

Expected: clear local workflow and CI targets.

## 2) Security guardrail (1 minute)

```bash
make lint-secrets
```

Expected: secret-commit guard passes.

## 3) PR-equivalent gate (2–4 minutes)

```bash
make ci-fast
```

Expected: format/lint/typecheck/smoke/docs/examples all pass with non-mutating behavior.

## 4) Full gate (3–5 minutes)

```bash
make ci
```

Expected: full test matrix passes, live tests skipped by default for deterministic offline runs.

## 5) Showcase docs and release discipline (1 minute)

Open:
- `docs/architecture.md`
- `docs/ci.md`
- `docs/reviewer-verification.md`
- `docs/public-release-runbook.md`

## Troubleshooting

- If `make ci-fast` fails, rerun failed stage directly (shown in output)
- If live tests are needed: `CI_LIVE_TESTS_MODE=required make ci`
- If secrets guard fails, untrack forbidden files before commit
