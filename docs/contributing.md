# Contributing to Splunk TUI

Thank you for your interest in contributing!

## IDE Setup

### Visual Studio Code
We provide a pre-configured `.vscode` directory. When you open the project, VS Code will suggest installing recommended extensions.

- **Rust Analyzer**: Used for completions and inline diagnostics.
- **CodeLLDB**: Used for debugging.

### JetBrains (CLion / RustRover)
The project includes run configurations in `.idea/runConfigurations`. You should see `Debug splunk-cli` and `Debug splunk-tui` in your run/debug menu.

## Development Workflow

1.  **Install dependencies**: `make install`
2.  **Run tests**: `make test`
3.  **Check your changes**: `make ci`
4.  **Auto-rebuild**: Use `cargo watch` for real-time feedback. A `.cargo-watch.json` is provided for configuration.
