# Splunk TUI - Repository Guidelines

## Project Overview

Splunk TUI is a Rust-based terminal interface for interacting with Splunk Enterprise. It provides both a CLI (`splunk-cli`) and a TUI (`splunk-tui`) built on the ratatui framework, with a focus on security, type safety, and testability.

## Core Principles

1. **Simplicity First**: The project uses straightforward Rust patterns with clear separation of concerns. Each crate has a single responsibility.

2. **Type Safety**: Leverage Rust's type system for compile-time guarantees. Use `thiserror` for library error types and `anyhow` for application error propagation.

3. **Secure by Default**: Credentials are handled with `secrecy` crate. Never log sensitive information. Session tokens are stored securely and auto-renewed.

4. **Testability**: All HTTP interactions are mockable. Unit tests cover business logic; integration tests verify API contracts.

5. **Error Clarity**: Errors provide actionable context. Include relevant details (endpoint, status code, suggested fix) in error messages.

## Project Structure

```
splunk-tui/
├── crates/
│   ├── cli/          # CLI binary (splunk-cli)
│   ├── tui/          # TUI binary (splunk-tui)
│   ├── client/       # Splunk API client library
│   └── config/       # Configuration management
├── docs/             # User documentation
├── scripts/          # Utility scripts
├── Makefile          # Build and development commands
└── Cargo.toml        # Workspace configuration
```

### Module Organization

- **crates/client**: Core Splunk REST API client with authentication, search jobs, and resource endpoints
- **crates/config**: Configuration loading from files, environment, and CLI args with keyring support
- **crates/cli**: Command-line interface with clap-based argument parsing and multiple commands
- **crates/tui**: Terminal user interface using ratatui with multiple screens and keyboard navigation

### Feature Parity & Code Reuse

**CRITICAL**: Maintain feature parity between CLI and TUI interfaces. When adding new functionality:

1. **Prioritize the client library**: Implement core business logic and API interactions in `crates/client`. Both CLI and TUI should use the same client functions.
2. **Reuse across interfaces**: Avoid duplicating logic between CLI and TUI. If a feature exists in one, it should be available in both unless there's a technical constraint preventing it.
3. **UI-specific logic only**: The `crates/cli` and `crates/tui` crates should only contain presentation layer code (parsing, formatting, event handling).
4. **Test once**: Shared client code reduces duplication in tests and ensures consistent behavior across interfaces.

Example: When adding "list apps" functionality:
- Implement `list_apps()` in `crates/client/src/endpoints/apps.rs`
- Call `client.list_apps()` from both `splunk-cli apps list` and TUI Apps screen
- Only CLI argument parsing or TUI rendering should differ

### Dependencies

- Test fixtures are in `crates/client/fixtures/` organized by endpoint type
- Integration tests are in `crates/*/tests/` with descriptive naming (`*_tests.rs`)
- TUI snapshots are in `crates/tui/tests/snapshots/` for UI regression testing

## Build, Test, and Development Commands

### Development Workflow

```bash
# Install dependencies (fetch only)
make install

# Update all dependencies to latest versions
make update

# Format code (writes changes)
make format

# Type check without building
make type-check

# Run linters (clippy with warnings as errors)
make lint

# Run all tests (unit + integration)
make test

# Run only unit tests
make test-unit

# Run only integration tests (HTTP mocking)
make test-integration

# Run live tests (requires running Splunk server at 192.168.1.122:8089)
make test-live

# Build release binaries and install to ~/.local/bin
make release
# Alias: make build

# Full CI pipeline (install → format → generate → lint → type-check → test → release)
make ci

# Clean build artifacts
make clean
```

### Critical CI Rules

- **NEVER** commit with failing CI. All tests must pass before completing implementation.
- `make ci` must pass before any commit. This is the local gate.
- Transient failures (e.g., test isolation issues) are still failures that require fixes.
- If CI fails, fix it before considering the task done.

## Coding Style & Naming Conventions

### Rust Style Guidelines

- Use `cargo fmt` for formatting (run `make format` before commits)
- Use `clippy` with warnings as errors (`make lint` must pass)
- Follow Rust API guidelines for public APIs
- Use `rustfmt.toml` for project-specific formatting rules (if present)

### Naming Patterns

- **Modules**: `snake_case` (e.g., `search.rs`, `auth.rs`)
- **Types**: `PascalCase` (e.g., `SplunkClient`, `SearchJob`)
- **Functions**: `snake_case` (e.g., `execute_search`, `get_config`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RETRIES`)
- **Tests**: Append `_tests.rs` for test modules (e.g., `jobs_tests.rs`)

### Error Handling Patterns

Library errors use `thiserror`:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Session expired, please re-authenticate")]
    SessionExpired,
}
```

Application errors use `anyhow` for flexible error propagation.

## Testing Guidelines

### Test Structure

- **Unit tests**: Located in source files within `#[cfg(test)]` modules
- **Integration tests**: Separate files in `crates/*/tests/` directory
- **Live tests**: Marked with `#[ignore]`, require running Splunk server

### Running Specific Tests

```bash
# Run tests for a specific crate
cargo test -p splunk-client

# Run a specific test file
cargo test -p splunk-cli --test jobs_tests

# Run a specific test by name
cargo test -p splunk-client test_search_job_creation

# Run only non-ignored tests
cargo test --lib

# Run only ignored (live) tests
cargo test --ignored
```

### Test Naming Conventions

- Test functions use `test_` prefix for clarity
- Integration test files end with `_tests.rs` (e.g., `jobs_tests.rs`)
- Mock fixtures in `crates/client/fixtures/` organized by resource type

### Coverage Requirements

- All public functions must have unit tests
- API interactions must have integration tests with mocked responses
- Error paths must be tested
- Critical paths (authentication, search execution) require live tests

## Commit & Pull Request Guidelines

### Commit Messages

- Use conventional commit format: `type(scope): description`
- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`
- Examples:
  - `feat(cli): add saved-searches run command`
  - `fix(client): handle session expiration gracefully`
  - `refactor(config): centralize profile loading logic`

### Pull Request Requirements

- PR description must include:
  - Clear summary of changes
  - Link to related issues (if any)
  - Testing performed
  - Breaking changes (if any)
- All CI checks must pass
- Code review required before merge
- Update `docs/usage.md` for any CLI command changes or TUI keyboard shortcuts
- Ensure CLI `--help` remains in sync with implementation

## Documentation

### Code Documentation

- Every module should have a top-level docstring explaining its purpose
- Public APIs must have documentation comments (`///`)
- Use examples in docstrings where helpful

### User Documentation

- **CLI help**: Run `splunk-cli --help` and `splunk-cli <command> --help`
- **TUI help**: Press `?` within the TUI
- **Usage guide**: See `docs/usage.md` for comprehensive usage documentation
- **User guide**: See `docs/user-guide.md` for detailed user instructions

### Documentation Sync

- When adding CLI commands or modifying CLI arguments, update `docs/usage.md`
- When adding TUI screens or keyboard shortcuts, update `docs/usage.md`
- Keep CLI `--help` output in sync with actual implementation

## Architecture

```
┌─────────────────────────────────────────┐
│         User Interface                  │
│  ┌──────────────┐  ┌──────────────┐    │
│  │ CLI (clap)   │  │ TUI (ratatui)│   │
│  └──────┬───────┘  └──────┬───────┘    │
└─────────┼──────────────────┼───────────┘
          └────────┬─────────┘
                   │
┌─────────────────────────────────────────┐
│      Application Logic Layer            │
│  Command Handlers / State Machine       │
└───────────────────┬─────────────────────┘
                    │
┌─────────────────────────────────────────┐
│         Splunk Client Layer             │
│  - Auth (session/API token)             │
│  - Search jobs & results                │
│  - Cluster management                   │
│  - Index operations                     │
└───────────────────┬─────────────────────┘
                    │
┌─────────────────────────────────────────┐
│       Configuration Layer               │
│  Environment, files, CLI args           │
└─────────────────────────────────────────┘
```

## Splunk API Integration

### Authentication
- **Session Token**: Username/password login, session stored and auto-renewed
- **API Token**: Bearer token authentication (preferred for automation)

### Rate Limiting
- Implement exponential backoff for 429 responses
- Default: 3 retries with 1s, 2s, 4s delays

### Search Jobs
- Jobs are created asynchronously
- Poll for completion with exponential backoff
- Results fetched in pages (default: 1000 rows)

## Known Constraints

- Splunk Enterprise v9+ REST API
- Minimum Rust version: 1.84
- TLS 1.2+ required for HTTPS connections
- Session tokens expire after 1 hour of inactivity

## Future Enhancements

These are NOT in scope for initial release:
- Distributed search across multiple Splunk instances
- Real-time search updates
- Advanced alerting configuration
- Custom visualization dashboards
