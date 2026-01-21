.PHONY: install update lint type-check format clean test build release generate ci help

# Binaries and Installation
BINS := splunk splunk-tui
INSTALL_DIR := ~/.local/bin

# Default target
.DEFAULT_GOAL := help

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
test: test-unit test-integration

# Run unit tests (lib and bins)
test-unit:
	cargo test --lib --bins

# Run integration tests
test-integration:
	cargo test -p splunk-client --test integration_tests
	cargo test -p splunk-cli --test health_tests
	cargo test -p splunk-cli --test jobs_tests
	cargo test -p splunk-cli --test kvstore_tests
	cargo test -p splunk-tui --test app_tests
	cargo test -p splunk-tui --test snapshot_tests

# Run live tests (requires Splunk server at 192.168.1.122:8089)
test-live:
	cargo test -p splunk-client --test live_tests -- --ignored

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

# No code generation required for this project
generate:
	@echo "No code generation required for this project."

# CI pipeline: install -> format -> generate -> lint -> type-check -> test -> release
ci: install format generate lint type-check test release

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
	@echo "  make test             - Run all tests (unit + integration)"
	@echo "  make test-unit        - Run unit tests (lib and bins)"
	@echo "  make test-integration - Run integration tests (HTTP mocking)"
	@echo "  make test-live        - Run live tests (requires Splunk server)"
	@echo "  make test-live-manual - Run manual live server test script"
	@echo "  make release          - Optimized release build and install to $(INSTALL_DIR)"
	@echo "  make build            - Alias for release"
	@echo "  make generate         - No code generation required for this project"
	@echo "  make ci               - Run full CI pipeline (install -> format -> generate -> lint -> type-check -> test -> release)"
	@echo "  make help             - Show this help message"
