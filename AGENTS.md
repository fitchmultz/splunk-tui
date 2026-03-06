# Splunk TUI - Agent Briefing

This file is an index, not a semantic source. Do not hardcode behavior, defaults, or contracts here.

## Project Intent
- Rust workspace with shared Splunk client logic and two frontends:
- `splunk-cli` for command-line workflows.
- `splunk-tui` for interactive terminal workflows.
- Verification is local-only. The repository should not carry hosted CI workflow configs or assume external automation services.

## Source of Truth Index
- Build/test/CI contract: `Makefile`
- Workspace/package topology: `Cargo.toml`, `crates/*/Cargo.toml`
- Toolchain/lint/format policy: `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.cargo/config.toml`
- Client API behavior/models/errors: `crates/client/src/**`
- TUI behavior/keybindings/rendering: `crates/tui/src/**`
- CLI command surface/help: `crates/cli/src/**`
- Config/env schema and persisted state: `crates/config/src/**`, `.env.example`, `.env.test.example`
- Architecture constraints: `crates/architecture-tests/tests/**`
- User/developer docs: `README.md`, `docs/index.md`, `docs/architecture.md`, `docs/ci.md`, `docs/validation-checklist.md`, `docs/testing.md`, `docs/usage.md`, `docs/user-guide.md`
- Secret scanning guardrails: `scripts/check-secrets.sh`, `make lint-secrets`
- Help-output env redaction controls: `crates/cli/src/args.rs`, `crates/tui/src/cli.rs`
- Visual regression and accessibility contracts: `crates/tui/tests/helpers.rs`, `crates/tui/tests/snapshot_styled_tests.rs`, `crates/tui/tests/interaction_render_tests.rs`, `crates/tui/tests/accessibility_contrast_tests.rs`, `make tui-visual`, `make tui-accessibility`

## Agent Rules For Drift Control
1. Do not encode operational semantics in this file.
2. When behavior changes, update the owning source-of-truth files first, then tests/docs.
3. Keep this file to pointers and high-level intent only.
4. If files or ownership move, update references in this index in the same change.
5. Before completion, run the repo gate from `Makefile` (`make ci`, plus required mode flags).

## Implementation Reminder
- Preserve CLI/TUI parity by placing shared behavior in `crates/client` and consuming it from both UIs.
