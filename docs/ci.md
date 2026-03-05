# CI Strategy and Resource Governance

This repository uses a **two-tier local-only quality model**.

- **Fast local gate:** `make ci-fast`
- **Full local gate:** `make ci`

Both gates are deterministic, non-mutating, and avoid binary install side effects.

## Check Tiers

### Fast Local Gate (`make ci-fast`)

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

### Full Local Gate (`make ci`)

```bash
make ci
```

`make ci` extends the fast gate with:

- full workspace test suite (`make test`)
- live-test policy hook (`LIVE_TESTS_MODE=$(CI_LIVE_TESTS_MODE) make test-live`, default `skip`)
- CI-profile binary build (`make build PROFILE=ci`)

### Additional Local Validation

```bash
# strict live integration validation (requires Splunk instance)
CI_LIVE_TESTS_MODE=required make ci

# chaos and resilience-focused tests
make test-chaos

# benchmark profiling
make bench
```

## Resource Controls

The Makefile exposes explicit pressure controls:

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
