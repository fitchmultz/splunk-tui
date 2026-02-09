.PHONY: install update lint format clean type-check \
	test test-all test-unit test-integration test-chaos test-live test-live-manual \
	bench bench-client bench-cli bench-tui \
	build release generate lint-docs ci help lint-secrets install-hooks \
	_generate-docs _lint-docs-check examples-test

# Binaries and Installation
BINS := splunk-cli splunk-tui generate-tui-docs
INSTALL_DIR ?= $(HOME)/.local/bin

# Build profile: 'release' (default) or 'ci' (faster, less optimized)
PROFILE ?= release

# Map profile to target directory name
ifeq ($(PROFILE),release)
  TARGET_DIR := release
else
  TARGET_DIR := $(PROFILE)
endif

# Default target
.DEFAULT_GOAL := help

# Hermetic tests:
# Prevent test runs from accidentally loading a developer's local `.env` file.
# (The Rust config loader respects `DOTENV_DISABLED` in `crates/config/src/loader.rs`.)
test test-all test-unit test-integration test-chaos ci: export DOTENV_DISABLED=1 CARGO_TERM_VERBOSE=false

# Fetch all dependencies (warm caches, no build)
install:
	@echo "→ Fetching deps (locked)..."
	@cargo fetch
	@echo "  ✓ Deps fetched"

# Update all dependencies to latest stable versions
update:
	@cargo update

# Format code with rustfmt (write mode)
format:
	@echo "→ Formatting code..."
	@cargo fmt --all
	@echo "  ✓ Formatting complete"

# Run clippy (two-phase: autofix then strict check) and then verify formatting (cheap)
lint:
	@echo "→ Clippy autofix (phase 1/2)..."
	@cargo clippy --fix --allow-dirty --workspace --all-targets --all-features --locked
	@echo "→ Clippy strict check (phase 2/2)..."
	@cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
	@echo "→ Format check..."
	@cargo fmt --all --check
	@echo "  ✓ Lint complete"

# Run cargo check (fast compilation check without producing binaries)
type-check:
	@echo "→ Type checking..."
	@cargo check --workspace --all-targets --all-features --locked
	@echo "  ✓ Type check complete"

# Run secret-commit guard
lint-secrets:
	@bash scripts/check-secrets.sh

# Install local git pre-commit hook
install-hooks:
	@ln -sf ../../scripts/check-secrets.sh .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Git pre-commit hook installed (pointing to scripts/check-secrets.sh)"

# Remove build artifacts (do NOT delete Cargo.lock if you care about speed)
clean:
	@cargo clean
	@rm -rf target/

# Run all tests
test: test-all

# Run all tests (workspace, all targets). This is the default "everything" gate.
test-all:
	@cargo test --workspace --all-targets --all-features --locked

# Run unit tests (lib and bins)
test-unit:
	@cargo test --workspace --lib --bins --all-features --locked

# Run integration tests
test-integration:
	@cargo test --workspace --tests --all-features --locked

# Run chaos engineering tests
# These tests verify resilience under network failures, partial responses,
# timing issues, and rapid state changes.
test-chaos:
	@echo "→ Running chaos engineering tests..."
	@cargo test -p splunk-client --test chaos_network_tests --features test-utils --locked
	@cargo test -p splunk-client --test chaos_timing_tests --features test-utils --locked
	@cargo test -p splunk-client --test chaos_flapping_tests --features test-utils --locked
	@echo "  ✓ Chaos tests complete"

# Run live tests (requires a reachable Splunk server configured via env / .env.test)
test-live:
	@if [ "$$SKIP_LIVE_TESTS" = "1" ]; then \
		echo "Skipping live tests (SKIP_LIVE_TESTS=1)"; \
	else \
		cargo test -p splunk-client --test live_tests --all-features --locked -- --ignored; \
		cargo test -p splunk-cli --test live_tests --all-features --locked -- --ignored; \
	fi

# Manual live server test script
test-live-manual:
	@bash scripts/test-live-server.sh

# Benchmark targets
# Run all benchmarks
bench:
	@echo "→ Running all benchmarks..."
	@cargo bench --workspace

# Run client crate benchmarks
bench-client:
	@echo "→ Running client benchmarks..."
	@cargo bench -p splunk-client

# Run CLI crate benchmarks  
bench-cli:
	@echo "→ Running CLI benchmarks..."
	@cargo bench -p splunk-cli

# Run TUI crate benchmarks
bench-tui:
	@echo "→ Running TUI benchmarks..."
	@cargo bench -p splunk-tui

# Release build and install binaries (required every time)
# Usage: make release [PROFILE=ci] (PROFILE defaults to 'release')
release:
	@echo "→ Release build (profile: $(PROFILE))..."
	@cargo build --profile $(PROFILE) --workspace --bins --all-features --locked
	@mkdir -p $(INSTALL_DIR)
	@for bin in $(BINS); do \
		echo "Installing $$bin to $(INSTALL_DIR)..."; \
		install -m 0755 target/$(TARGET_DIR)/$$bin $(INSTALL_DIR)/$$bin; \
	done
	@echo "  ✓ Release build + install complete"

# Build target (alias for release)
build: release

# Regenerate derived documentation artifacts (internal: no release dependency)
_generate-docs:
	@echo "→ Generating derived docs..."
	@$(INSTALL_DIR)/generate-tui-docs
	@echo "  ✓ Generated"

# Verify documentation is up to date (internal: no release dependency)
_lint-docs-check:
	@echo "→ Checking docs drift..."
	@$(INSTALL_DIR)/generate-tui-docs --check
	@echo "  ✓ Docs clean"

# Regenerate derived documentation artifacts
# IMPORTANT: use the already-built release binary to avoid a second debug compile.
generate: release _generate-docs

# Verify documentation is up to date
# IMPORTANT: use the already-built release binary to avoid a second debug compile.
lint-docs: release _lint-docs-check

# CI pipeline (local speed-first):
# deps -> format -> lint-secrets -> clippy fix + fmt check -> type-check -> tests -> live tests -> release+install -> docs generate/check
#
# Notes:
# - release is required every time, and we reuse that binary for generate/lint-docs.
# - Uses PROFILE=ci for faster builds (still produces working binaries).
ci:
	@echo "→ Local CI (mutates code, builds+installs with ci profile)..."
	@echo ""
	@set -e; \
	$(MAKE) install              || { echo ""; echo "✗ CI failed at: install"; exit 1; }; \
	$(MAKE) format               || { echo ""; echo "✗ CI failed at: format"; exit 1; }; \
	$(MAKE) lint-secrets         || { echo ""; echo "✗ CI failed at: lint-secrets"; exit 1; }; \
	$(MAKE) lint                 || { echo ""; echo "✗ CI failed at: lint"; exit 1; }; \
	$(MAKE) type-check           || { echo ""; echo "✗ CI failed at: type-check"; exit 1; }; \
	$(MAKE) test                 || { echo ""; echo "✗ CI failed at: test"; exit 1; }; \
	$(MAKE) test-live            || { echo ""; echo "✗ CI failed at: test-live"; exit 1; }; \
	$(MAKE) release PROFILE=ci   || { echo ""; echo "✗ CI failed at: release"; exit 1; }; \
	$(MAKE) _lint-docs-check     || { echo ""; echo "✗ CI failed at: lint-docs"; exit 1; }; \
	$(MAKE) examples-test        || { echo ""; echo "✗ CI failed at: examples-test"; exit 1; }
	@echo ""
	@echo "✓ CI completed successfully"

# Validate example scripts (syntax check and executable permissions)
examples-test:
	@echo "→ Validating example scripts..."
	@find examples -name "*.sh" -type f | while read script; do \
		echo "  Checking $$script..."; \
		bash -n "$$script" || { echo ""; echo "✗ Syntax error in $$script"; exit 1; }; \
		[ -x "$$script" ] || { echo ""; echo "✗ Not executable: $$script"; exit 1; }; \
	done
	@echo "  ✓ All example scripts validated"

# Display help for each target
help:
	@echo "Splunk TUI - Available targets:"
	@echo ""
	@echo "  make install          - Fetch all dependencies (locked)"
	@echo "  make update           - Update all dependencies to latest stable versions"
	@echo "  make format           - Format code with rustfmt (write mode)"
	@echo "  make lint             - Clippy autofix + format check"
	@echo "  make type-check       - Type check the workspace (cargo check)"
	@echo "  make clean            - Remove build artifacts (keeps Cargo.lock)"
	@echo "  make test             - Run all tests (workspace, all targets)"
	@echo "  make test-all         - Alias for make test"
	@echo "  make test-unit        - Run unit tests (lib and bins)"
	@echo "  make test-integration - Run integration tests"
	@echo "  make test-chaos       - Run chaos engineering tests"
	@echo "  make test-live        - Run live tests (set SKIP_LIVE_TESTS=1 to skip)"
	@echo "  make test-live-manual - Run manual live server test script"
	@echo "  make bench            - Run all benchmarks"
	@echo "  make bench-client     - Run client crate benchmarks"
	@echo "  make bench-cli        - Run CLI crate benchmarks"
	@echo "  make bench-tui        - Run TUI crate benchmarks"
	@echo "  make release          - Release build (bins) and install to $(INSTALL_DIR)"
	@echo "  make build            - Alias for release"
	@echo "  make generate         - Regenerate derived docs (via installed release binary)"
	@echo "  make lint-docs        - Verify docs are up to date (via installed release binary)"
	@echo "  make lint-secrets     - Run secret-commit guard"
	@echo "  make install-hooks    - Install git pre-commit hook for secret guard"
	@echo "  make ci               - Full local pipeline (speed-first, mutates code, uses ci profile for faster builds)"
	@echo "  make help             - Show this help message"
