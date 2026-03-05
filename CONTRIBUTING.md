# Contributing

Thanks for contributing to `splunk-tui`.

For full contributor documentation, see [docs/contributing.md](./docs/contributing.md).

## Quick Dev Loop

```bash
make install
make format
make lint
make test
LIVE_TESTS_MODE=optional make ci
```

## Notes

- `make ci` is the local quality gate before opening a PR.
- For faster TUI iteration, use `make tui-smoke`.
