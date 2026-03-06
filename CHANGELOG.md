# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Validation and operations docs (`docs/index.md`, `docs/architecture.md`, `docs/ci.md`, `docs/validation-checklist.md`).
- Architecture tests for markdown link integrity, exit-code docs drift, and forbidden tracked artifacts.

### Changed

- `make ci` is now non-mutating and avoids binary-install side effects.
- CI live-test behavior now uses `CI_LIVE_TESTS_MODE` (default `skip`) for deterministic offline gates.
- `docs/containers.md` exit-code mapping now matches `splunk-cli` contract.
- Contributor docs expanded with explicit check/fix loops and resource controls.
