# Architecture Overview

This project is a Rust workspace with shared Splunk integration logic and two user interfaces:

- `splunk-cli` for automation and scripting
- `splunk-tui` for interactive terminal operations

The design goal is **single implementation of Splunk behavior** with multiple frontends.

## Workspace Components

### `crates/client` (`splunk-client`)

Shared Splunk REST API client and domain models.

Responsibilities:

- Authentication (session + API token)
- Retry/backoff and circuit-breaker behavior
- Endpoint request/response logic
- Domain models and error taxonomy
- Metrics/tracing hooks

Not responsible for:

- UI rendering
- CLI argument parsing
- Persisted profile editing UX

### `crates/config` (`splunk-config`)

Configuration loading and persistence.

Responsibilities:

- Layered config resolution (CLI args, env, profile, defaults)
- Profile persistence and migration
- Keyring/encryption integration
- Search/internal-log defaults and persisted UI state support

### `crates/cli` (`splunk-cli`)

Automation-first command interface.

Responsibilities:

- Command parsing and validation
- Stable exit-code contract for scripts
- Structured output modes (table/json/csv/xml/etc.)
- Wiring config + client into command handlers

### `crates/tui` (`splunk-tui`)

Interactive terminal application.

Responsibilities:

- App state transitions and screen navigation
- Event/input handling and keybindings
- Rendering and popup flows
- Async side-effect orchestration against `splunk-client`

## High-Level Flow

```text
User (CLI flags / TUI input)
        |
        v
+---------------------+
| Frontend Layer      |
| - crates/cli        |
| - crates/tui        |
+----------+----------+
           |
           v
+---------------------+
| Config Layer        |
| - crates/config     |
+----------+----------+
           |
           v
+---------------------+
| API Client Layer    |
| - crates/client     |
+----------+----------+
           |
           v
      Splunk REST API
```

## Error Handling and Automation Contract

Error classification is centralized in `splunk-client` and translated to automation-friendly exit codes in `splunk-cli`.

- Client-level category: `FailureCategory`
- CLI-level contract: `ExitCode` enum (0,1,2,3,4,5,6,7,8,130)

This keeps user-facing diagnostics and script behavior predictable.

## Testing and Quality Guardrails

The repo uses layered testing:

- Unit tests inside crates
- Integration tests in crate `tests/` directories
- Chaos tests for network/timing/flapping resilience
- Snapshot tests for TUI UX regression
- `architecture-tests` for repo-level invariants (Makefile contract, docs drift, file-size policy, hygiene)

## CI and Local Quality Model

The local quality contract is split into two gates:

- `make ci-fast` for fast local validation
- `make ci` for full local pre-release validation

Design goals for both gates:

- Non-mutating checks
- Deterministic offline defaults
- Resource-governed execution (`CARGO_JOBS`, `RUST_TEST_THREADS`)
- Clear stage failures

## Key Trade-offs

1. **Comprehensive local gate vs runtime**
   - A full local gate increases runtime, but catches cross-crate regressions early.
   - Resource controls are used to avoid machine saturation.

2. **Feature coverage vs shipping minimal binaries**
   - Checks still compile/test broad functionality.
   - Shipping build targets avoid unnecessary feature bloat.

3. **Rich docs vs maintenance overhead**
   - Docs are split by concern (usage/testing/containers/workflows/CI/review readiness).
   - Architecture tests enforce link integrity and reduce drift risk.
