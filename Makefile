.PHONY: install update lint type-check format clean test test-all test-unit test-integration test-live test-live-manual build release generate ci help lint-secrets install-hooks

# Binaries and Installation
BINS := splunk-cli splunk-tui
INSTALL_DIR := ~/.local/bin

# Default target
.DEFAULT_GOAL := help

# Hermetic tests:
# Prevent test runs from accidentally loading a developer's local `.env` file.
# (The Rust config loader respects `DOTENV_DISABLED` in `crates/config/src/loader.rs`.)
test test-all test-unit test-integration ci: export DOTENV_DISABLED=1

# Fetch all dependencies (does not install binaries)
install:
	cargo fetch

# Update all dependencies to latest stable versions
update:
	cargo update

# Run clippy and format check
lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo fmt --all --check

# Run secret-commit guard
lint-secrets:
	bash scripts/check-secrets.sh

# Install local git pre-commit hook
install-hooks:
	ln -sf ../../scripts/check-secrets.sh .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
	@echo "Git pre-commit hook installed (pointing to scripts/check-secrets.sh)"

# Type check all crates
type-check:
	cargo check --workspace --all-targets --all-features

# Format code with rustfmt (write mode)
format:
	cargo fmt --all

# Remove build artifacts and lock files
clean:
	cargo clean
	rm -f Cargo.lock
	rm -rf target/

# Run all tests
test: test-all

# Run all tests (workspace, all targets). This is the default "everything" gate.
test-all:
	cargo test --workspace --all-targets --all-features

# Run unit tests (lib and bins)
test-unit:
	cargo test --workspace --lib --bins --all-features

# Run integration tests
test-integration:
	cargo test --workspace --tests --all-features

# Run live tests (requires a reachable Splunk server configured via env / .env.test)
test-live:
	@if [ "$$SKIP_LIVE_TESTS" = "1" ]; then \
		echo "Skipping live tests (SKIP_LIVE_TESTS=1)"; \
		exit 0; \
	fi
	cargo test --workspace --all-targets --all-features -- --ignored

# Manual live server test script
test-live-manual:
	bash scripts/test-live-server.sh

# Release build and install binaries
release:
	cargo build --release --all-features
	mkdir -p $(INSTALL_DIR)
	@for bin in $(BINS); do \
		echo "Installing $$bin to $(INSTALL_DIR)..."; \
		cp target/release/$$bin $(INSTALL_DIR)/; \
	done

# Build target (alias for release)
build: release

# Regenerate derived documentation artifacts
generate:
	cargo run -p splunk-tui --bin generate-tui-docs

# CI pipeline: install -> format -> lint-secrets -> generate -> lint -> type-check -> test -> test-live -> release
ci: install format lint-secrets generate lint type-check test test-live release

# Display help for each target
help:
	@echo "Splunk TUI - Available targets:"
	@echo ""
	@echo "  make install          - Fetch all dependencies"
	@echo "  make update           - Update all dependencies to latest stable versions"
	@echo "  make lint             - Run clippy (warnings as errors) and format check"
	@echo "  make type-check       - Run cargo check"
	@echo "  make format           - Format code with rustfmt (write mode)"
	@echo "  make clean            - Remove build artifacts and lock files"
	@echo "  make test             - Run all tests (workspace, all targets)"
	@echo "  make test-all         - Alias for make test"
	@echo "  make test-unit        - Run unit tests (lib and bins)"
	@echo "  make test-integration - Run integration tests"
	@echo "  make test-live        - Run live tests (requires Splunk server; set SKIP_LIVE_TESTS=1 to skip)"
	@echo "  make test-live-manual - Run manual live server test script"
	@echo "  make release          - Optimized release build and install to $(INSTALL_DIR)"
	@echo "  make build            - Alias for release"
	@echo "  make generate         - Regenerate derived documentation (TUI keybindings)"
	@echo "  make lint-secrets     - Run secret-commit guard (fail if sensitive files are tracked)"
	@echo "  make install-hooks    - Install git pre-commit hook for secret guard"
	@echo "  make ci               - Run full CI pipeline (install -> format -> lint-secrets -> generate -> lint -> type-check -> test -> test-live -> release)"
	@echo "  make help             - Show this help message"
