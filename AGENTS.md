# Splunk TUI - Repository Guidelines

Splunk TUI is a Rust workspace providing two frontends over a shared Splunk REST client:
- `splunk-cli`: clap-based CLI
- `splunk-tui`: ratatui-based interactive TUI

The project optimizes for security, type safety, and testability. Keep behavior consistent between CLI and TUI by implementing core logic in the client library and reusing it from both UIs.

## Repository Layout

```
splunk-tui/
├── crates/
│   ├── architecture-tests/  # Architecture constraint validation (e.g., dependency rules)
│   ├── client/              # Splunk REST API client (shared business logic)
│   ├── cli/                 # CLI presentation layer (splunk-cli binary)
│   ├── config/              # Configuration loading and validation
│   └── tui/                 # TUI presentation layer (splunk-tui binary)
├── docs/                    # User + development docs
│   ├── usage.md             # Usage guide for CLI and TUI
│   └── user-guide.md        # Detailed user documentation
├── scripts/                 # Utility scripts (live/manual workflows)
├── Makefile                 # Local dev + CI contract (use this)
├── Cargo.toml               # Workspace configuration
├── rust-toolchain.toml      # Rust toolchain pin (1.84)
└── .cargo/config.toml       # Build optimizations
```

### Module Relationships

- **`crates/client`**: Core business logic. All API calls, data models, and shared utilities live here.
- **`crates/config`**: Configuration management. Used by both CLI and TUI for consistent config loading.
- **`crates/cli`**: Thin presentation layer. Parses CLI arguments and calls into `client`.
- **`crates/tui`**: Interactive UI layer. Handles terminal rendering and user input; delegates to `client`.
- **`crates/architecture-tests`**: Enforces architectural constraints (e.g., ensuring CLI/TUI don't bypass client).

## Non-Negotiables

- **Treat `make ci` as the local gate before merging.** The CI pipeline is the source of truth; don't rely on remote CI to catch issues.
- **Never log or print secrets** (passwords, session tokens, API tokens). Use `secrecy` types and avoid `Debug`/`Display` on sensitive values.
- **Maintain CLI/TUI feature parity**: If a feature ships in one UI, it must ship in the other (via shared `crates/client` code) unless there's a documented constraint.
- **Prefer shared abstractions in `crates/client`** over duplicated CLI/TUI logic.
- **Hermetic tests**: Tests must not depend on local `.env` files. `make test` and `make ci` run with `DOTENV_DISABLED=1`.

## Build, Test, and Development Commands

The `Makefile` is the primary interface for development tasks. Always use `make` commands rather than running `cargo` directly to ensure consistent behavior.

### Core Commands

| Command | Description |
|---------|-------------|
| `make install` | Fetch all dependencies (locked) |
| `make format` | Format code with rustfmt (write mode) |
| `make lint` | Run clippy autofix + format check |
| `make type-check` | Fast type check without producing binaries |
| `make test` | Run all tests (workspace, all targets) |
| `make test-unit` | Run unit tests (lib and bins) |
| `make test-integration` | Run integration tests |
| `make test-live` | Run live tests against a Splunk server |
| `make release` | Build release binaries and install to `~/.local/bin` |
| `make generate` | Regenerate TUI keybinding documentation |
| `make lint-docs` | Verify documentation is up to date |
| `make ci` | Full CI pipeline (local gate before merging) |

### Notes

- `make ci` uses `PROFILE=ci` for faster builds (still produces working binaries).
- `make clean` does NOT delete `Cargo.lock` (kept for reproducible builds and speed).
- Set `SKIP_LIVE_TESTS=1` to skip live server tests: `make ci SKIP_LIVE_TESTS=1`.
- Integration tests are discovered automatically; adding `crates/*/tests/*.rs` requires no Makefile updates.

## Coding Standards

### Language & Tooling

- **Toolchain**: Rust `1.84` (managed via `rust-toolchain.toml`).
- **Formatting**: `rustfmt` with project-level config in `rustfmt.toml`.
- **Linting**: `clippy` with strict warnings (`-D warnings`) configured in `clippy.toml`.
- **Errors**: Use `thiserror` for library errors and `anyhow` at the application edge.
- **Public API**: Keep it small; prefer `pub(crate)` unless cross-crate use is required.

### Naming Conventions

- **Modules**: Use `snake_case` (e.g., `job_manager.rs`).
- **Types/Traits**: Use `PascalCase` (e.g., `JobManager`).
- **Functions/Variables**: Use `snake_case` (e.g., `fetch_jobs()`).
- **Constants**: Use `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_TIMEOUT`).
- **Test files**: Place in `tests/` directory or use `#[cfg(test)]` modules with suffix `_tests.rs`.

### Feature Parity & Reuse (Critical)

When adding a feature:
1. Implement API and business logic in `crates/client` first.
2. Call that shared code from both `crates/cli` and `crates/tui`.
3. Keep UI crates limited to parsing/formatting/rendering/event handling.

## Testing Guidelines

### Test Organization

- **Unit tests**: Use `#[cfg(test)]` modules near the code they test.
- **Integration tests**: Place in `crates/*/tests/*.rs` (prefer one concept per file, e.g., `jobs_tests.rs`).
- **Live tests**: Mark with `#[ignore]`; run via `make test-live`. Configure via `.env.test` or environment variables.
- **Fixtures**: Store in `crates/client/fixtures/` (organized by endpoint/resource).
- **TUI regression**: Use snapshots in `crates/tui/tests/snapshots/`.

### Running Targeted Tests

```bash
cargo test -p splunk-client --test integration_tests
cargo test -p splunk-cli --test jobs_tests
cargo test -p splunk-tui --test snapshot_tests
```

### Hermetic Test Rule

`make test` and `make ci` run with `DOTENV_DISABLED=1`, so workspace/root `.env` files are **not** loaded during tests.

- If a test needs to validate dotenv behavior, it must explicitly unset `DOTENV_DISABLED` for the spawned process.
- Live tests should use `.env.test` (copy from `.env.test.example`) or environment variables; avoid hardcoding server addresses in code or docs.

## Configuration & Secrets

### Local Development

- Copy `.env.example` to `.env` for local development settings.
- For tests, copy `.env.test.example` to `.env.test`.

### Environment Variables

| Variable | Description |
|----------|-------------|
| `SPLUNK_BASE_URL` | Splunk REST API URL (e.g., `https://localhost:8089`) |
| `SPLUNK_USERNAME` | Splunk username |
| `SPLUNK_PASSWORD` | Splunk password |
| `SPLUNK_API_TOKEN` | Splunk API token (preferred over user/pass) |
| `SPLUNK_SKIP_VERIFY` | Skip TLS verification (`true`/`false`) |
| `SPLUNK_TIMEOUT` | Connection timeout in seconds |
| `SPLUNK_MAX_RETRIES` | Maximum retries for failed requests |

### Security Practices

- Credentials flow through `secrecy` types; never use raw `String` for secrets.
- Avoid implementing `Debug` or `Display` on types containing secrets.
- Use `scripts/check-secrets.sh` (or `make lint-secrets`) to prevent accidental commits of secrets.

## Documentation

### CLI Documentation

- Ensure `splunk-cli --help` stays accurate.
- Update `docs/usage.md` when adding or changing CLI commands.

### TUI Documentation

- Update `docs/usage.md` for TUI changes.
- Keep the in-app `?` help consistent with documentation.

### Auto-Generated Keybindings

TUI keybinding documentation is auto-generated from the keymap source:

```bash
make generate   # Regenerate after modifying crates/tui/src/input/keymap.rs
make lint-docs  # Verify docs are in sync (runs in CI)
```

Markers `<!-- BEGIN TUI KEYBINDINGS -->` and `<!-- END TUI KEYBINDINGS -->` delimit generated sections in:
- `README.md`
- `docs/usage.md`
- `docs/user-guide.md`

## Commits & Reviews

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(cli): add job deletion command
fix(client): handle 503 rate limit responses
docs: update installation instructions
refactor(tui): simplify event handling
```

Common scopes: `cli`, `client`, `config`, `tui`, `arch` (architecture-tests), `docs`.

### Pull Request Guidelines

- Run `make ci` before submitting and include a short testing note in the PR description.
- Link related issues using `Fixes #123` or `Relates to #456`.
- Keep changes focused; prefer multiple small PRs over one large PR.

## Tooling & Build Configuration

### Optional but Recommended Tools

- **sccache**: Configured in `.cargo/config.toml` as `rustc-wrapper` for faster incremental builds.
- **lld linker**: Configured for Linux and macOS targets in `.cargo/config.toml`.

If these tools are unavailable, the configuration gracefully degrades to default Rust behavior.

### Configuration Files

| File | Purpose |
|------|---------|
| `rust-toolchain.toml` | Pins Rust version to 1.84 with required components |
| `rustfmt.toml` | Project-level rustfmt configuration |
| `clippy.toml` | Project-level clippy configuration (MSRV, lint levels) |
| `.cargo/config.toml` | Build optimizations: wrapper, parallel jobs, linker settings, dev profile |
| `Cargo.toml` | Workspace configuration including `[profile.release]` and `[profile.ci]` |

### Build Profiles

- **dev** (`.cargo/config.toml`): Fast builds with `opt-level = 0`, `codegen-units = 256`
- **release** (`Cargo.toml`): Optimized builds with `opt-level = 3`, `lto = true`, `strip = true`, `panic = "abort"`
- **ci** (`Cargo.toml`): CI-optimized with `opt-level = 2`, `lto = false`, `codegen-units = 16`

## Constraints & Defaults

### API Constraints

- **Splunk Enterprise v9+** REST API required.
- **TLS 1.2+** required for all connections.

### Session & Rate Limiting

- Session tokens expire after ~1 hour of inactivity.
- Default rate limiting: exponential backoff with 3 retries (1s/2s/4s delays).

### Default Configuration Values

| Setting | Default | Description |
|---------|---------|-------------|
| Session TTL | 3600s | Session token validity period |
| Session Expiry Buffer | 60s | Proactive refresh before expiry |
| Health Check Interval | 60s | TUI health polling frequency |
| Search Max Results | 1000 | Default result limit for searches |
| Internal Logs Count | 100 | Default log entries for internal queries |
