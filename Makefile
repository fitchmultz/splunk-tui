.PHONY: install update lint format clean type-check \
	test test-all test-unit test-integration test-chaos test-live test-live-manual \
	bench bench-client bench-cli bench-tui \
	build release generate lint-docs ci help lint-secrets install-hooks \
	_generate-docs _lint-docs-check examples-test \
	tui-smoke run-tui \
	docker-build docker-run-cli docker-run-tui docker-compose-up \
	docker-compose-cli docker-compose-tui docker-clean \
	helm-install helm-upgrade helm-uninstall

# Binaries and Installation
BINS := splunk-cli splunk-tui generate-tui-docs
INSTALL_DIR ?= $(HOME)/.local/bin

# Build profile: 'release' (default) or 'ci' (faster, less optimized)
PROFILE ?= release

# Live tests mode: 'required' (CI default), 'optional' (local dev), 'skip' (explicit bypass)
LIVE_TESTS_MODE ?= required

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
test test-all test-unit test-integration test-chaos tui-smoke ci: export DOTENV_DISABLED=1 CARGO_TERM_VERBOSE=false

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

# Run all tests (workspace, libs/bins/tests only - excludes benchmarks).
# Use `make bench` for performance benchmarks.
test-all:
	@cargo test --workspace --lib --bins --tests --all-features --locked

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

# Run TUI UX smoke tests (snapshot-based regression suite)
# Fast feedback for UI/UX changes - runs only snapshot tests from the TUI crate.
# Use this for rapid iteration on popups, layouts, and visual components.
# For full validation before merging, run `make ci`.
tui-smoke:
	@echo "→ Running TUI UX smoke tests..."
	@cargo test -p splunk-tui \
		--test snapshot_tutorial_tests \
		--test snapshot_error_details_tests \
		--test snapshot_popups_tests \
		--test snapshot_footer_tests \
		--test snapshot_screens_tests \
		--test snapshot_search_tests \
		--test snapshot_jobs_tests \
		--test snapshot_misc_tests \
		--all-features --locked
	@echo "  ✓ TUI smoke tests complete"

# Run the TUI binary locally (developer convenience)
# Builds and runs the TUI with the dev profile for quick manual testing.
# Requires SPLUNK_* environment variables to be set (e.g., via .env file).
run-tui:
	@echo "→ Running TUI locally..."
	@cargo run --package splunk-tui --bin splunk-tui --all-features

# Run live tests (requires a reachable Splunk server configured via env / .env.test)
# Mode controlled by LIVE_TESTS_MODE: required|optional|skip
# - required: Fail if env/server unavailable (CI default)
# - optional: Skip with warning if unavailable (local dev)
# - skip: Explicit bypass
test-live:
	@echo "→ Running live tests (mode: $(LIVE_TESTS_MODE))..."
	@mode="$(LIVE_TESTS_MODE)"; \
	case "$$mode" in \
		skip) \
			echo "  $(YELLOW)Skipping live tests (LIVE_TESTS_MODE=skip)$(NC)"; \
			;; \
		*) \
			./scripts/validate-live-test-env.sh "$$mode"; \
			code=$$?; \
			if [ $$code -eq 2 ]; then \
				echo "  $(YELLOW)Live tests skipped (optional mode)$(NC)"; \
			elif [ $$code -eq 1 ]; then \
				echo ""; \
				echo "✗ Live tests failed: environment not configured for required mode"; \
				exit 1; \
			else \
				echo "  Environment validated, running tests..."; \
				cargo test -p splunk-client --test live_tests --all-features --locked -- --ignored && \
				cargo test -p splunk-cli --test live_tests --all-features --locked -- --ignored; \
			fi \
			;; \
	esac
	@echo "  ✓ Live tests complete"

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
	LIVE_TESTS_MODE=$(LIVE_TESTS_MODE) $(MAKE) test-live || { echo ""; echo "✗ CI failed at: test-live"; exit 1; }; \
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
	@echo "  make test             - Run all tests (workspace, libs/bins/tests only)"
	@echo "  make test-all         - Alias for make test"
	@echo "  make test-unit        - Run unit tests (lib and bins)"
	@echo "  make test-integration - Run integration tests"
	@echo "  make test-chaos       - Run chaos engineering tests"
	@echo "  make test-live        - Run live tests (LIVE_TESTS_MODE=required|optional|skip)"
	@echo "  make test-live-manual - Run manual live server test script"
	@echo "  make tui-smoke        - Run TUI UX smoke tests (snapshot suite, fast feedback)"
	@echo "  make run-tui          - Run TUI locally for manual testing"
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
	@echo "  make docker-build     - Build Docker image locally"
	@echo "  make docker-run-cli   - Run CLI in Docker container"
	@echo "  make docker-run-tui   - Run TUI in Docker container (interactive)"
	@echo "  make docker-compose-up - Start docker-compose services (Splunk)"
	@echo "  make docker-compose-cli - Run CLI via docker-compose"
	@echo "  make docker-compose-tui - Run TUI via docker-compose (interactive)"
	@echo "  make docker-clean     - Remove local Docker images"
	@echo "  make helm-install     - Install Helm chart"
	@echo "  make helm-upgrade     - Upgrade Helm release"
	@echo "  make helm-uninstall   - Uninstall Helm release"
	@echo "  make help             - Show this help message"

# Docker Operations
# ------------------------------------------------------------------------------

# Build Docker image locally
docker-build:
	@echo "→ Building Docker image..."
	@docker build -t splunk-tui:latest .
	@echo "  ✓ Docker image built (splunk-tui:latest)"

# Run CLI in container (use ARGS="" to pass additional arguments)
docker-run-cli:
	@echo "→ Running CLI in Docker container..."
	@docker run --rm -it \
		-e SPLUNK_BASE_URL \
		-e SPLUNK_USERNAME \
		-e SPLUNK_PASSWORD \
		-e SPLUNK_API_TOKEN \
		-e SPLUNK_SKIP_VERIFY \
		-e SPLUNK_TIMEOUT \
		-e SPLUNK_MAX_RETRIES \
		-e RUST_LOG \
		splunk-tui:latest $(ARGS)
	@echo "  ✓ CLI container exited"

# Run TUI in container (interactive)
docker-run-tui:
	@echo "→ Running TUI in Docker container..."
	@docker run --rm -it \
		-e SPLUNK_BASE_URL \
		-e SPLUNK_USERNAME \
		-e SPLUNK_PASSWORD \
		-e SPLUNK_API_TOKEN \
		-e SPLUNK_SKIP_VERIFY \
		-e SPLUNK_TIMEOUT \
		-e SPLUNK_MAX_RETRIES \
		-e RUST_LOG \
		--entrypoint /usr/local/bin/splunk-tui \
		splunk-tui:latest $(ARGS)
	@echo "  ✓ TUI container exited"

# Start docker-compose services (Splunk only)
docker-compose-up:
	@echo "→ Starting docker-compose services..."
	@docker-compose up -d splunk
	@echo "  ✓ Services started"
	@echo ""
	@echo "  Splunk is starting up. Health check will run automatically."
	@echo "  Web UI: http://localhost:8000"
	@echo "  REST API: https://localhost:8089"

# Run CLI via docker-compose
docker-compose-cli:
	@echo "→ Running CLI via docker-compose..."
	@docker-compose --profile cli run --rm cli $(ARGS)
	@echo "  ✓ CLI service exited"

# Run TUI via docker-compose (interactive)
docker-compose-tui:
	@echo "→ Running TUI via docker-compose..."
	@docker-compose --profile tui run --rm tui $(ARGS)
	@echo "  ✓ TUI service exited"

# Remove local docker images
docker-clean:
	@echo "→ Removing local Docker images..."
	@docker rmi splunk-tui:latest 2>/dev/null || echo "  Image not found or already removed"
	@docker-compose down --rmi local 2>/dev/null || true
	@echo "  ✓ Docker cleanup complete"

# Helm Operations
# ------------------------------------------------------------------------------

# Install Helm chart
helm-install:
	@if [ -z "$${SPLUNK_BASE_URL}" ]; then echo "Error: SPLUNK_BASE_URL not set"; exit 1; fi
	@if [ -z "$${SPLUNK_USERNAME}" ]; then echo "Error: SPLUNK_USERNAME not set"; exit 1; fi
	@if [ -z "$${SPLUNK_PASSWORD}" ]; then echo "Error: SPLUNK_PASSWORD not set"; exit 1; fi
	@echo "→ Installing Helm chart..."
	@helm install splunk-tui ./helm/splunk-tui \
		--set cli.splunk.baseUrl=$${SPLUNK_BASE_URL} \
		--set cli.splunk.username=$${SPLUNK_USERNAME} \
		--set cli.splunk.password=$${SPLUNK_PASSWORD}
	@echo "  ✓ Helm chart installed"

# Upgrade Helm release
helm-upgrade:
	@if [ -z "$${SPLUNK_BASE_URL}" ]; then echo "Error: SPLUNK_BASE_URL not set"; exit 1; fi
	@if [ -z "$${SPLUNK_USERNAME}" ]; then echo "Error: SPLUNK_USERNAME not set"; exit 1; fi
	@if [ -z "$${SPLUNK_PASSWORD}" ]; then echo "Error: SPLUNK_PASSWORD not set"; exit 1; fi
	@echo "→ Upgrading Helm release..."
	@helm upgrade splunk-tui ./helm/splunk-tui \
		--set cli.splunk.baseUrl=$${SPLUNK_BASE_URL} \
		--set cli.splunk.username=$${SPLUNK_USERNAME} \
		--set cli.splunk.password=$${SPLUNK_PASSWORD}
	@echo "  ✓ Helm release upgraded"

# Uninstall Helm release
helm-uninstall:
	@echo "→ Uninstalling Helm release..."
	@helm uninstall splunk-tui
	@echo "  ✓ Helm release uninstalled"
