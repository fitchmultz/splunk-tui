# Cookbook Pattern: Deterministic Local-First CI

## Problem

Teams need fast PR confidence without expensive or flaky default pipelines.

## Pattern

Split local CI into:

- **Fast gate (`make ci-fast`)**: high-signal, deterministic, no side effects
- **Full gate (`make ci`)**: broader validation, still deterministic offline by default

## Why it works

- Fast feedback for contributors
- Full coverage for mainline confidence
- Predictable resource usage through explicit knobs (`CARGO_JOBS`, `RUST_TEST_THREADS`)

## Safe Defaults

- No auto-fixing in required gates
- No binary installation side effects in CI gates
- Live tests explicitly controlled (`CI_LIVE_TESTS_MODE`)

## Commands

```bash
make ci-fast
make ci
CI_LIVE_TESTS_MODE=required make ci
```

## Trade-offs

- Fast gate is intentionally incomplete (smoke-focused)
- Full gate is heavier but still local and reproducible
