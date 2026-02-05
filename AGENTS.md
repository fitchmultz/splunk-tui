# Splunk TUI - Repository Guidelines

Splunk TUI is a Rust workspace providing two frontends over a shared Splunk REST client:
- `splunk-cli`: clap-based CLI
- `splunk-tui`: ratatui-based interactive TUI

The project optimizes for security, type safety, and testability. Keep behavior consistent between CLI and TUI by implementing core logic in the client library and reusing it from both UIs.

## Repository Layout

```
splunk-tui/
├── crates/
│   ├── client/       # Splunk REST API client (shared business logic)
│   ├── config/       # Configuration loading and validation
│   ├── cli/          # CLI presentation layer (splunk-cli)
│   └── tui/          # TUI presentation layer (splunk-tui)
├── docs/             # User + development docs
├── scripts/          # Utility scripts (live/manual workflows)
├── Makefile          # Local dev + CI contract (use this)
└── rust-toolchain.toml # Rust toolchain pin (1.84)
```

## Non-Negotiables

- Treat `make ci` as the local gate before merging.
- Never log or print secrets (passwords, session tokens, API tokens).
- Maintain CLI/TUI feature parity: if it ships in one UI, it should ship in the other (shared via `crates/client`) unless there’s a documented constraint.
- Prefer shared abstractions in `crates/client` over duplicated CLI/TUI logic.

## Configuration & Secrets

- Local config: `.env` (copy from `.env.example`), tests may use `.env.test`.
- Typical env vars: `SPLUNK_BASE_URL`, `SPLUNK_USERNAME`, `SPLUNK_PASSWORD`, `SPLUNK_API_TOKEN`, `SPLUNK_SKIP_VERIFY`.
- Credentials should flow through `secrecy` types; avoid `Debug`/`Display` on secret values.

## Makefile Contract (Commands You Should Use)

```bash
make install      # cargo fetch
make format       # cargo fmt (write mode)
make lint         # clippy -D warnings + cargo fmt --check
make type-check   # cargo check (workspace)
make test         # all tests (workspace, all targets)
make test-live    # live tests (ignored tests) against dev server
make test-live-manual # runs scripts/test-live-server.sh
make release      # build --release and install to ~/.local/bin
make ci           # install -> format -> lint-secrets -> lint-docs -> lint -> type-check -> test -> test-live -> release
```

Notes:
- `make clean` does NOT delete `Cargo.lock` (kept for reproducible builds and speed).
- `make test-live` runs all `#[ignore]` tests across the workspace. Set `SKIP_LIVE_TESTS=1` to skip when the dev server is unavailable.
- Live tests should be configured via `.env.test` (untracked; copy from `.env.test.example`) or environment variables; avoid hardcoding server addresses in code or docs.
- Integration tests are discovered automatically; adding a new `crates/*/tests/*.rs` file requires no Makefile updates.

## Coding Standards

- Toolchain: Rust `1.84` (see `rust-toolchain.toml`).
- Errors: use `thiserror` for library errors and `anyhow` at the application edge.
- Public API: keep it small; prefer `pub(crate)` unless cross-crate use is required.

### Feature Parity & Reuse (Critical)

When adding a feature:
1. Implement API and business logic in `crates/client` first.
2. Call that shared code from both `crates/cli` and `crates/tui`.
3. Keep UI crates limited to parsing/formatting/rendering/event handling.

## Testing

- **Hermetic test rule:** `make test` / `make ci` run with `DOTENV_DISABLED=1`, so workspace/root `.env` files are **not** loaded during tests.
  - If a test needs to specifically validate dotenv behavior, it must explicitly **unset** `DOTENV_DISABLED` (or set it to a non-disabling value) for the spawned process.
- Unit tests: `#[cfg(test)]` modules near the code.
- Integration tests: `crates/*/tests/*` (prefer one concept per file, e.g. `jobs_tests.rs`).
- Fixtures: `crates/client/fixtures/` (organized by endpoint/resource).
- TUI regression: snapshots in `crates/tui/tests/snapshots/`.

Run targeted tests:
```bash
cargo test -p splunk-client --test integration_tests
cargo test -p splunk-cli --test jobs_tests
cargo test -p splunk-tui --test snapshot_tests
```

## Documentation Updates

- CLI changes: ensure `splunk-cli --help` stays correct and update `docs/usage.md`.
- TUI changes: update `docs/usage.md` and keep the in-app `?` help consistent.
- **TUI keybindings are auto-generated**: The keybinding documentation in `README.md`, `docs/usage.md`, and `docs/user-guide.md` is automatically generated from the keymap source.
  - Run `make generate` to regenerate keybindings after modifying `crates/tui/src/input/keymap.rs`.
  - Run `make lint-docs` to verify documentation is in sync (runs in CI via `make ci`).
  - Markers `<!-- BEGIN TUI KEYBINDINGS -->` and `<!-- END TUI KEYBINDINGS -->` delimit the generated sections.

## Commits & Reviews

- Commit messages: use Conventional Commits (`feat(cli): ...`, `fix(client): ...`, `docs: ...`).
- Before merging: run `make ci` and include a short testing note in the PR/summary.
- This repo treats `make ci` as the source of truth; don’t rely on remote CI to catch issues.

## Tooling & Build Configuration

This section documents the tooling setup required for development and CI.

### Required Tools

- **Rust 1.84** (managed via `rustup` using `rust-toolchain.toml`)
- **sccache** (optional but recommended): Configured in `.cargo/config.toml` as `rustc-wrapper` for faster incremental builds
- **lld** linker (optional but recommended): Configured for Linux and macOS targets in `.cargo/config.toml`

If sccache or lld are not available, the configuration gracefully degrades to default Rust behavior.

### Configuration Files

| File | Purpose |
|------|---------|
| `rust-toolchain.toml` | Pins Rust version to 1.84 with required components (rustfmt, clippy, rust-src) |
| `rustfmt.toml` | Project-level rustfmt configuration for consistent formatting |
| `clippy.toml` | Project-level clippy configuration (MSRV, lint levels) |
| `.cargo/config.toml` | Build optimizations: sccache wrapper, parallel jobs, linker settings, dev profile |
| `Cargo.toml` | Workspace configuration including `[profile.release]` and `[profile.ci]` |

### Build Profiles

- **dev** (`.cargo/config.toml`): Fast builds with `opt-level = 0`, `codegen-units = 256`
- **release** (`Cargo.toml`): Optimized builds with `opt-level = 3`, `lto = true`, `strip = true`, `panic = "abort"`
- **ci** (`Cargo.toml`): CI-optimized release with `opt-level = 2`, `lto = false`, `codegen-units = 16` for faster builds

### Skipping Live Tests

Set `SKIP_LIVE_TESTS=1` to skip live server tests:
```bash
make ci SKIP_LIVE_TESTS=1
```

## Constraints & Defaults

- Splunk Enterprise v9+ REST API; TLS 1.2+ required.
- Session tokens expire after ~1 hour of inactivity.
- Rate limiting: exponential backoff; default retries are 3 with 1s/2s/4s delays.
