# Release Readiness Report

## Scope

This report tracks public-release hardening work aimed at reducing reviewer friction for production-focused engineering roles.

## Top 10 Risks and Mitigations

1. **Local CI mutated source files**
   - **Change:** `make ci` now uses non-mutating checks (`format-check`, `lint-check`) instead of auto-fix targets.

2. **Local CI installed binaries into user home directories**
   - **Change:** `make ci` now builds with `make build PROFILE=ci` and no longer installs binaries.

3. **Live tests made CI non-deterministic on machines without Splunk**
   - **Change:** introduced `CI_LIVE_TESTS_MODE` with default `skip` in `make ci`.

4. **Resource pressure during CI/test loops**
   - **Change:** introduced split gates (`make ci-fast` for PRs, `make ci` for full validation), plus explicit resource knobs (`CARGO_JOBS`, `RUST_TEST_THREADS`, `CI_LIVE_TESTS_MODE`).

5. **Unclear docs entrypoint for reviewers**
   - **Change:** added `docs/index.md` as a structured documentation hub.

6. **Missing concise architecture narrative for external reviewers**
   - **Change:** added `docs/architecture.md` with component boundaries, flow, and trade-offs.

7. **PR and heavy validation were not clearly separated**
   - **Change:** added `docs/ci.md`, moved PR workflow to `make ci-fast`, and added `.github/workflows/ci-full.yml` for push-main/nightly/manual full validation.

8. **No formal reviewer playbook for validation**
   - **Change:** added `docs/reviewer-verification.md` with concrete commands.

9. **Inconsistent container exit-code docs vs CLI contract**
   - **Change:** updated `docs/containers.md` exit-code table to match `crates/cli/src/error.rs`.

10. **Contributor onboarding lacked complete local workflow**
    - **Change:** expanded `docs/contributing.md` and refreshed `CONTRIBUTING.md` with check/fix loops and CI expectations.

## Remaining Known Issues / Follow-ups

1. **Public history cutover not yet executed**
   - Existing private history still includes mixed commit styles.
   - **Next step:** execute `docs/public-release-runbook.md` before flipping repository visibility to public.

2. **Fixture generator script quality**
   - `scripts/generate-fixtures.sh` still needs end-to-end implementation quality review.
   - **Next step:** either fully implement deterministic generation + check mode or remove the script if unused.

## Before/After DX Notes

### Before

- `make ci` performed auto-fixes and user-directory binary installation.
- CI policy did not clearly separate PR-speed checks from full validation.
- Live-test behavior in CI context was less explicit for offline environments.
- Documentation lacked a single index and dedicated reviewer checklist.

### After

- `make ci-fast` provides a bounded PR gate; `make ci` remains the full mainline/nightly gate.
- `make ci` is non-mutating and side-effect-free for binary installation.
- Live tests are explicitly controlled through `CI_LIVE_TESTS_MODE`.
- Architecture, CI strategy, verification checklist, and readiness report are now first-class docs.
- Contributor docs now provide clear check loop vs fix loop guidance.
