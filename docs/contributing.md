# Contributing to Splunk TUI

Thank you for your interest in contributing!

## Prerequisites

- Rust 1.84+
- Make

## IDE Setup

### Visual Studio Code
We provide a pre-configured `.vscode` directory. When you open the project, VS Code will suggest installing recommended extensions.

- **Rust Analyzer**: Used for completions and inline diagnostics.
- **CodeLLDB**: Used for debugging.

### JetBrains (CLion / RustRover)
The project includes run configurations in `.idea/runConfigurations`. You should see `Debug splunk-cli` and `Debug splunk-tui` in your run/debug menu.

The tracked `.vscode/` and `.idea/runConfigurations/` files are intentional to improve out-of-the-box onboarding for contributors.

## Development Workflow

1. **Install dependencies**: `make install`
2. **Fast local loop**: `make format && make lint && make test`
3. **Full gate**: `LIVE_TESTS_MODE=optional make ci`
4. **Auto-rebuild**: use `cargo watch` for real-time feedback (a `.cargo-watch.json` is included)

## Snapshot Workflow (TUI)

- Run smoke snapshots quickly: `make tui-smoke`
- Update snapshots when expected: `cargo insta review`

## Optional Build Speedups

These are optional and should not be required for first-time contributors:

- `sccache`: install and set `RUSTC_WRAPPER=sccache`
- `lld`: configure target-specific linker flags in local (untracked) Cargo config overrides
