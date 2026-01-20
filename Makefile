.PHONY: install update lint type-check format clean test build generate ci help

# Default target
.DEFAULT_GOAL := help

# Fetch all dependencies (does not install binaries)
install:
	cargo fetch

# Update all dependencies to latest stable versions
update:
	cargo update

# Run clippy with warnings as errors
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Type check all crates
type-check:
	cargo check --all-targets --all-features

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

# Run live tests (requires Splunk server at 192.168.1.122:8089)
test-live:
	cargo test -p splunk-client --test live_tests -- --ignored

# Manual live server test script
test-live-manual:
	bash scripts/test-live-server.sh

# Release build and install binaries to ~/.local/bin
build:
	cargo build --release --all-features
	mkdir -p ~/.local/bin
	cp target/release/splunk ~/.local/bin/
	cp target/release/splunk-tui ~/.local/bin/

# No code generation required for this project
generate:
	@echo "No code generation required for this project."

# CI pipeline: install -> format -> generate -> lint -> type-check -> test -> build
ci: install format generate lint type-check test build

# Display help for each target
help:
	@echo "Splunk TUI - Available targets:"
	@echo ""
	@echo "  make install          - Fetch all dependencies"
	@echo "  make update           - Update all dependencies to latest stable versions"
	@echo "  make lint             - Run clippy (warnings as errors)"
	@echo "  make type-check       - Run cargo check"
	@echo "  make format           - Format code with rustfmt (write mode)"
	@echo "  make clean            - Remove build artifacts and lock files"
	@echo "  make test             - Run all tests (unit + integration)"
	@echo "  make test-unit        - Run unit tests (lib and bins)"
	@echo "  make test-integration - Run integration tests (HTTP mocking)"
	@echo "  make test-live        - Run live tests (requires Splunk server)"
	@echo "  make test-live-manual - Run manual live server test script"
	@echo "  make build            - Release build and install binaries to ~/.local/bin"
	@echo "  make generate         - No code generation required for this project"
	@echo "  make ci               - Run full CI pipeline (install -> format -> generate -> lint -> type-check -> test -> build)"
	@echo "  make help             - Show this help message"
