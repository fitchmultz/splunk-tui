# Release Readiness Report

## Scope

Public-release hardening with emphasis on correctness, deterministic CI, visual/UX confidence, and secret-safety.

## What changed in this hardening pass

1. **Style-aware visual regression gate for TUI**
   - Added `snapshot_styled_tests.rs` to capture styled buffer runs (`fg/bg/modifiers`) and assert semantic color contracts.
   - Added `interaction_render_tests.rs` for keyboard-driven render/state transitions.
2. **Accessibility contrast validation**
   - Added `accessibility_contrast_tests.rs` to enforce minimum contrast thresholds across all shipped themes.
3. **Fast local gate contract enforcement**
   - Added `visual_testing_contract_tests.rs` in architecture-tests to ensure Makefile visual targets remain wired into smoke CI.
4. **CI/Docs wiring**
   - Added `make tui-visual` and `make tui-accessibility` and wired both into `make test-smoke`.
   - Updated testing and validation docs accordingly.

## Evidence (executed on March 5, 2026)

### Deterministic local gates

- `make tui-visual` ✅
- `make tui-accessibility` ✅
- `make test-smoke` ✅
- `make ci-fast` ✅
- `make ci` ✅

### Strict live validation against real Splunk

Live target used: `https://192.168.1.122:8089` with provided isolated test credentials.

- `LIVE_TESTS_MODE=required make test-live` ✅
  - splunk-client live tests: **21/21 passed**
  - splunk-cli live tests: **15/15 passed**
- `CI_LIVE_TESTS_MODE=required make ci` ✅

### Live API/CLI JSON shape receipts

Executed live commands and captured outputs under `logs/validation/`:

- `live_health.json`
- `live_indexes.json`
- `live_apps.json`
- `live_jobs.json`
- `live_search.json`
- `live_doctor.json`

Validated successfully:

- health payload includes `server_info`, `splunkd_health`, `license_usage`, `kvstore_status`
- list endpoints produce non-empty typed arrays with expected fields
- search returns expected row payload (`{"foo": "qual-check"}`)
- doctor output includes connectivity/config checks

## Confidence by concern

- API response/runtime correctness (covered flows): **High**
- TUI resize robustness: **High** (resize suite in full CI + stress tests)
- TUI visual semantics regression resistance: **High** (character snapshots + styled snapshots + interaction checks)
- User friction / intuitiveness: **Medium-High** (strong automated and live smoke coverage; still inherently benefits from additional external user trials)

## Top remaining risks

1. **Long-tail UX workflows**
   - Core flows are strongly covered, but subjective usability for uncommon workflows should continue via periodic dogfood runs.
2. **Environment-coupled live confidence**
   - Strict live confidence requires reachable Splunk infrastructure; the default fast local gate remains deterministic and offline by design.

## Reproduce locally

```bash
# deterministic fast local gate
make ci-fast

# full deterministic gate
make ci

# strict live gate (requires SPLUNK_* env)
CI_LIVE_TESTS_MODE=required make ci
```

Supporting artifacts:

- `docs/validation-checklist.md`
- `docs/validation-receipts.md`
- `docs/dogfood-2026-03-05.md`

## CI matrix summary

- **Fast local gate:** `make ci-fast`
  - Includes: format/lint/type-check, secrets, smoke tests, `tui-visual`, `tui-accessibility`, docs drift, examples.
  - Designed for deterministic, bounded local resource usage.
- **Full local gate:** `make ci`
  - Full workspace tests + optional/required live tests via `CI_LIVE_TESTS_MODE`.

## Public-release recommendation

Ready for public release based on executed evidence above.

Before flipping visibility, execute `docs/public-release-runbook.md`.
