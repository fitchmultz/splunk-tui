# Contributing

Thanks for contributing to `splunk-tui`.

For the full contributor guide, see [docs/contributing.md](./docs/contributing.md).

## Quick Start

```bash
make install
make build
make ci-fast
```

## Common Loops

```bash
# Non-mutating checks
make format-check
make lint-check
make test

# Auto-fix flow
make fix
make lint-check
```

## Notes

- `make ci-fast` is the PR-required local quality gate.
- `make ci` is the full gate used for mainline/nightly parity and pre-release checks.
- `make ci` defaults to `CI_LIVE_TESTS_MODE=skip` for deterministic offline checks.
- For strict live validation: `CI_LIVE_TESTS_MODE=required make ci`.
- For fast TUI iteration: `make tui-smoke`.
