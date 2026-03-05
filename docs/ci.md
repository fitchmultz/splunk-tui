# CI Strategy and Resource Governance

This repository uses a **two-tier local-first quality model**.

- **PR-required gate:** `make ci-fast`
- **Full release-grade gate:** `make ci`

Both gates are deterministic, non-mutating, and avoid binary install side effects.

## Workflow Mapping

- **`CI (Fast PR Gate)`** (`.github/workflows/ci.yml`)
  - Triggers: `pull_request`, `workflow_dispatch`
  - Runs: `make ci-fast`
- **`CI (Full)`** (`.github/workflows/ci-full.yml`)
  - Triggers: `push` to `main`, nightly schedule, `workflow_dispatch`
  - Runs: `make ci`
- **Docker workflow**
  - PR trigger narrowed to container-related file changes
  - PR builds run `linux/amd64` only
  - Source-only PRs may skip Docker build; container validation still runs on `main` and tag pushes
  - Main/tag builds remain multi-arch (`amd64`, `arm64`)

## Check Tiers

### PR-Required (`make ci-fast`)

```bash
make ci-fast
```

`make ci-fast` includes:

1. dependency fetch (`make install`)
2. format check (`make format-check`)
3. secret guard (`make lint-secrets`)
4. lint check (`make lint-check`)
5. type check (`make type-check`)
6. deterministic smoke test suite (`make test-smoke`), including:
   - `make tui-smoke` (character snapshots)
   - `make tui-visual` (style-aware snapshots + interaction visual checks)
   - `make tui-accessibility` (theme contrast checks)
7. docs drift check (`make _lint-docs-check PROFILE=ci`)
8. examples script validation (`make examples-test`)

### Full / Nightly / Mainline (`make ci`)

```bash
make ci
```

`make ci` extends the fast gate with:

- full workspace test suite (`make test`)
- live-test policy hook (`LIVE_TESTS_MODE=$(CI_LIVE_TESTS_MODE) make test-live`, default `skip`)
- CI-profile binary build (`make build PROFILE=ci`)

### Manual / On-Demand Heavy Validation

```bash
# strict live integration validation (requires Splunk instance)
CI_LIVE_TESTS_MODE=required make ci

# chaos and resilience-focused tests
make test-chaos

# benchmark profiling
make bench
```

## Resource Controls

The Makefile and CI workflows expose explicit pressure controls:

- `CARGO_JOBS` (default: `4`) — cargo build/test parallelism
- `RUST_TEST_THREADS` (default: `1`) — Rust test harness concurrency
- `CI_LIVE_TESTS_MODE` (default: `skip`) — live-test policy in full CI

Examples:

```bash
# lower CPU pressure when multitasking
CARGO_JOBS=2 RUST_TEST_THREADS=1 make ci-fast

# allow optional live tests in full gate
CI_LIVE_TESTS_MODE=optional make ci
```

## Expected Runtime (typical laptop, warm cache)

Approximate and hardware-dependent:

- `make tui-smoke`: ~0.5–1 min
- `make tui-visual`: ~0.5–2 min
- `make tui-accessibility`: <0.5 min
- `make test-smoke`: ~6–17 min
- `make ci-fast`: ~9–22 min
- `make ci`: ~20–60 min
- `CI_LIVE_TESTS_MODE=required make ci`: depends on Splunk environment latency/stability

## Failure Model

Both gates fail fast with stage-specific error messages.

If a stage fails, fix that stage directly rather than bypassing the gate.
