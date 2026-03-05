# Release Readiness Report

## Scope

Public-release hardening with emphasis on reviewer confidence in correctness, UX/DX quality, deterministic CI, and secret-safety.

## High-Risk Areas Addressed

1. **Secret exposure in CLI/TUI help output**
   - Env-backed flags now hide env values in help text (`hide_env_values = true`).
   - Regression tests added for both `splunk-cli` and `splunk-tui` help output.
2. **Plaintext config flow reliability**
   - `config set --plaintext` and `config edit --plaintext` now disable config-file encryption before save to prevent follow-up decryption/profile-load failures.
   - Regression test added for cross-invocation readability.
3. **Search UX mismatch on bare SPL queries**
   - Bare queries like `index=_internal | head 5` are normalized to `search ...` automatically.
   - Unit coverage added for normalization behavior.
4. **CI determinism and safety**
   - `make ci-fast`, `make ci`, and strict live mode all passing.
5. **Secret guardrails**
   - `make lint-secrets` enforced and passing.

## Current State Snapshot (2026-03-05)

Validated locally on **March 5, 2026**:

- `make lint-secrets` ✅
- `make ci-fast` ✅
- `make ci` ✅
- `CI_LIVE_TESTS_MODE=required make ci` ✅
  - Client live tests: **21/21**
  - CLI live tests: **15/15**

Qualitative dogfood evidence is captured in:
- `docs/role-evidence/qualitative-dogfood-2026-03-05.md`

## Confidence by Concern

- API response/runtime correctness (covered flows): **High**
- TUI resize robustness: **High** (automated resize suite + tmux stress run)
- First-user friction / intuitiveness: **Medium-High** (targeted fixes validated, but subjective UX always benefits from additional external user trials)

## Remaining Known Risks

1. **Environment-dependent live confidence**
   - Strict live coverage requires reachable Splunk and valid credentials; by design this is optional for default PR gates.
2. **Unvalidated paths outside covered flows**
   - Confidence is strongest for tested/dogfooded core journeys; long-tail workflows should continue to be expanded with live and UX checks.

## Public-Release Recommendation

Ready to go public from a quality and operational-discipline perspective, with evidence-backed confidence on core workflows and CI stability.

Before visibility flip, execute `docs/public-release-runbook.md`.
