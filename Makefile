# Purpose: Local automation and CI gate orchestration for the Splunk TUI Rust workspace.
# Responsibilities: Provide repeatable build, lint, test, release, and docs workflows.
# Scope: Local developer and workstation execution (no remote CI orchestration logic).
# Usage: Run `make help`; override knobs like `LIVE_TESTS_MODE`, `CARGO_JOBS`, and `RUST_TEST_THREADS` as needed.
# Invariants/Assumptions: Commands run from repo root with GNU Make and Cargo installed.

.PHONY: install update lint lint-fix lint-check fix format format-check clean type-check \
	test test-all test-unit test-integration test-smoke test-chaos test-live test-live-manual \
	bench bench-client bench-cli bench-tui \
	build install-bins release generate lint-docs ci ci-fast help lint-secrets install-hooks \
	_generate-docs _lint-docs-check examples-test \
	tui-smoke tui-visual tui-accessibility run-tui \
	docker-build docker-run-cli docker-run-tui docker-compose-up \
	docker-compose-cli docker-compose-tui docker-clean \
	helm-install helm-upgrade helm-uninstall

# Binaries and Installation
BINS := splunk-cli splunk-tui generate-tui-docs
INSTALL_DIR ?= $(HOME)/.local/bin

# Build profile: 'release' (default) or 'ci' (faster, less optimized)
PROFILE ?= release

# Live tests mode: 'optional' (local default), 'required' (strict gate), 'skip' (explicit bypass)
LIVE_TESTS_MODE ?= optional

# Local CI live-test mode (default skip for deterministic offline checks).
CI_LIVE_TESTS_MODE ?= skip

# Default cargo parallelism and test thread count (override as needed per host).
CARGO_JOBS ?= 4
RUST_TEST_THREADS ?= 1

CARGO_JOBS_FLAG := --jobs $(CARGO_JOBS)
TEST_ARGS := --test-threads $(RUST_TEST_THREADS)

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
test test-all test-unit test-integration test-smoke test-chaos tui-smoke tui-visual tui-accessibility ci ci-fast: export DOTENV_DISABLED=1 CARGO_TERM_VERBOSE=false

# Fetch all dependencies (warm caches, no build)
install:
	@echo "→ Fetching deps (locked)..."
	@cargo fetch --locked
	@echo "  ✓ Deps fetched"

# Update all dependencies to latest stable versions
update:
	@cargo update

# Format code with rustfmt (write mode)
format:
	@echo "→ Formatting code..."
	@cargo fmt --all
	@echo "  ✓ Formatting complete"

format-check:
	@echo "→ Format check..."
	@cargo fmt --all --check
	@echo "  ✓ Format check complete"

# Run clippy (two-phase: autofix then strict check) and then verify formatting (cheap)
lint:
	@echo "→ Clippy autofix (phase 1/2)..."
	@cargo clippy $(CARGO_JOBS_FLAG) --fix --allow-dirty --workspace --all-targets --all-features --locked
	@echo "→ Clippy strict check (phase 2/2)..."
	@cargo clippy $(CARGO_JOBS_FLAG) --workspace --all-targets --all-features --locked -- -D warnings
	@echo "→ Format check..."
	@cargo fmt --all --check
	@echo "  ✓ Lint complete"

lint-fix:
	@echo "→ Clippy autofix..."
	@cargo clippy $(CARGO_JOBS_FLAG) --fix --allow-dirty --workspace --all-targets --all-features --locked
	@echo "  ✓ Clippy autofix complete"

lint-check:
	@echo "→ Clippy strict check..."
	@cargo clippy $(CARGO_JOBS_FLAG) --workspace --all-targets --all-features --locked -- -D warnings
	@$(MAKE) format-check
	@echo "  ✓ Lint checks complete"

fix: format lint-fix

# Run cargo check (fast compilation check without producing binaries)
type-check:
	@echo "→ Type checking..."
	@cargo check $(CARGO_JOBS_FLAG) --workspace --all-targets --all-features --locked
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
	@cargo test $(CARGO_JOBS_FLAG) --workspace --lib --bins --tests --all-features --locked -- $(TEST_ARGS)

# Run unit tests (lib and bins)
test-unit:
	@cargo test $(CARGO_JOBS_FLAG) --workspace --lib --bins --all-features --locked -- $(TEST_ARGS)

# Run integration tests
test-integration:
	@cargo test $(CARGO_JOBS_FLAG) --workspace --tests --all-features --locked -- $(TEST_ARGS)

# Run deterministic smoke coverage (fast PR gate):
# - architecture/docs invariants
# - crate unit tests
# - minimal integration-path checks across CLI/client/config
# - TUI snapshot smoke suite
# - Style-aware visual checks and accessibility contrast checks
test-smoke:
	@echo "→ Running smoke test suite..."
	@cargo test $(CARGO_JOBS_FLAG) -p architecture-tests --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-client --lib --all-features --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test auth_tests --test server_tests --test search_tests --all-features --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-cli --bins --all-features --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-cli --test health_tests --test search_tests --all-features --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-config --lib --all-features --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-config --test integration_test --all-features --locked -- $(TEST_ARGS)
	@$(MAKE) tui-smoke
	@$(MAKE) tui-visual
	@$(MAKE) tui-accessibility
	@echo "  ✓ Smoke tests complete"

# Run chaos engineering tests
# These tests verify resilience under network failures, partial responses,
# timing issues, and rapid state changes.
test-chaos:
	@echo "→ Running chaos engineering tests..."
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test chaos_network_tests --features test-utils --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test chaos_timing_tests --features test-utils --locked -- $(TEST_ARGS)
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test chaos_flapping_tests --features test-utils --locked -- $(TEST_ARGS)
	@echo "  ✓ Chaos tests complete"

# Run TUI UX smoke tests (snapshot-based regression suite)
# Fast feedback for UI/UX changes - runs only snapshot tests from the TUI crate.
# Use this for rapid iteration on popups, layouts, and visual components.
# For full validation before merging, run `make ci`.
tui-smoke:
	@echo "→ Running TUI UX smoke tests..."
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-tui \
		--test snapshot_tutorial_tests \
		--test snapshot_error_details_tests \
		--test snapshot_popups_tests \
		--test snapshot_footer_tests \
		--test snapshot_screens_tests \
		--test snapshot_search_tests \
		--test snapshot_jobs_tests \
		--test snapshot_misc_tests \
		--all-features --locked -- $(TEST_ARGS)
	@echo "  ✓ TUI smoke tests complete"

# Run style-aware TUI visual tests (semantic colors/modifiers + interaction flows).
tui-visual:
	@echo "→ Running TUI visual style tests..."
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-tui \
		--test snapshot_styled_tests \
		--test interaction_render_tests \
		--all-features --locked -- $(TEST_ARGS)
	@echo "  ✓ TUI visual tests complete"

# Run accessibility contrast checks for supported color themes.
tui-accessibility:
	@echo "→ Running TUI accessibility contrast tests..."
	@cargo test $(CARGO_JOBS_FLAG) -p splunk-tui \
		--test accessibility_contrast_tests \
		--all-features --locked -- $(TEST_ARGS)
	@echo "  ✓ TUI accessibility checks complete"

# Run the TUI binary locally (developer convenience)
# Builds and runs the TUI with the dev profile for quick manual testing.
# Requires SPLUNK_* environment variables to be set (e.g., via .env file).
run-tui:
	@echo "→ Running TUI locally..."
	@cargo run $(CARGO_JOBS_FLAG) --package splunk-tui --bin splunk-tui --all-features

# Run live tests (requires a reachable Splunk server configured via env / .env.test)
# Mode controlled by LIVE_TESTS_MODE: required|optional|skip
# - required: Fail if env/server unavailable (strict CI/internal gates)
# - optional: Skip with warning if unavailable (default)
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
				cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test live_tests --all-features --locked -- --ignored $(TEST_ARGS) && \
				cargo test $(CARGO_JOBS_FLAG) -p splunk-cli --test live_tests --all-features --locked -- --ignored $(TEST_ARGS); \
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
	@cargo bench $(CARGO_JOBS_FLAG) --workspace

# Run client crate benchmarks
bench-client:
	@echo "→ Running client benchmarks..."
	@cargo bench $(CARGO_JOBS_FLAG) -p splunk-client

# Run CLI crate benchmarks  
bench-cli:
	@echo "→ Running CLI benchmarks..."
	@cargo bench $(CARGO_JOBS_FLAG) -p splunk-cli

# Run TUI crate benchmarks
bench-tui:
	@echo "→ Running TUI benchmarks..."
	@cargo bench $(CARGO_JOBS_FLAG) -p splunk-tui

# Build binaries only (no install side effects)
# Usage: make build [PROFILE=ci] (PROFILE defaults to 'release')
build:
	@echo "→ Building binaries (profile: $(PROFILE))..."
	@cargo build $(CARGO_JOBS_FLAG) --profile $(PROFILE) --workspace --bins --locked
	@echo "  ✓ Build complete"

# Install pre-built binaries to INSTALL_DIR
install-bins:
	@mkdir -p $(INSTALL_DIR)
	@for bin in $(BINS); do \
		echo "Installing $$bin to $(INSTALL_DIR)..."; \
		install -m 0755 target/$(TARGET_DIR)/$$bin $(INSTALL_DIR)/$$bin; \
	done
	@echo "  ✓ Binary install complete"

# Release build and install binaries (explicit side effect)
# Usage: make release [PROFILE=ci] (PROFILE defaults to 'release')
release: build install-bins
	@echo "  ✓ Release build + install complete"

# Regenerate derived documentation artifacts
_generate-docs:
	@echo "→ Generating derived docs..."
	@cargo run $(CARGO_JOBS_FLAG) --profile $(PROFILE) --package splunk-tui --bin generate-tui-docs --locked --
	@echo "  ✓ Generated"

# Verify documentation is up to date
_lint-docs-check:
	@echo "→ Checking docs drift..."
	@cargo run $(CARGO_JOBS_FLAG) --profile $(PROFILE) --package splunk-tui --bin generate-tui-docs --locked -- --check
	@echo "  ✓ Docs clean"

# Regenerate derived documentation artifacts
generate: _generate-docs

# Verify documentation is up to date
lint-docs: _lint-docs-check

# Fast local verification pipeline (deterministic, resource-governed):
# deps -> format-check -> lint-secrets -> clippy strict check -> type-check -> smoke tests -> docs check -> examples check
#
# Notes:
# - No live tests and no install side effects.
# - Optimized for rapid local feedback with bounded resource usage.
ci-fast:
	@echo "→ Local fast verification gate..."
	@echo ""
	@set -e; \
	$(MAKE) install              || { echo ""; echo "✗ CI (fast) failed at: install"; exit 1; }; \
	$(MAKE) format-check         || { echo ""; echo "✗ CI (fast) failed at: format-check"; exit 1; }; \
	$(MAKE) lint-secrets         || { echo ""; echo "✗ CI (fast) failed at: lint-secrets"; exit 1; }; \
	$(MAKE) lint-check           || { echo ""; echo "✗ CI (fast) failed at: lint-check"; exit 1; }; \
	$(MAKE) type-check           || { echo ""; echo "✗ CI (fast) failed at: type-check"; exit 1; }; \
	$(MAKE) test-smoke           || { echo ""; echo "✗ CI (fast) failed at: test-smoke"; exit 1; }; \
	$(MAKE) _lint-docs-check PROFILE=ci || { echo ""; echo "✗ CI (fast) failed at: lint-docs"; exit 1; }; \
	$(MAKE) examples-test        || { echo ""; echo "✗ CI (fast) failed at: examples-test"; exit 1; }
	@echo ""
	@echo "✓ Fast CI completed successfully"

# Full local verification pipeline:
# deps -> format-check -> lint-secrets -> clippy strict check -> type-check -> full tests -> live tests(mode=CI_LIVE_TESTS_MODE) -> build -> docs check -> examples check
#
# Notes:
# - Defaults LIVE tests to skip for deterministic offline validation.
# - Does not write fixes or install binaries into user directories.
# - Uses PROFILE=ci for faster build/doc-check compilation.
ci:
	@echo "→ Local full CI (non-mutating, no install side effects)..."
	@echo ""
	@set -e; \
	$(MAKE) install              || { echo ""; echo "✗ CI failed at: install"; exit 1; }; \
	$(MAKE) format-check         || { echo ""; echo "✗ CI failed at: format-check"; exit 1; }; \
	$(MAKE) lint-secrets         || { echo ""; echo "✗ CI failed at: lint-secrets"; exit 1; }; \
	$(MAKE) lint-check           || { echo ""; echo "✗ CI failed at: lint-check"; exit 1; }; \
	$(MAKE) type-check           || { echo ""; echo "✗ CI failed at: type-check"; exit 1; }; \
	$(MAKE) test                 || { echo ""; echo "✗ CI failed at: test"; exit 1; }; \
	LIVE_TESTS_MODE=$(CI_LIVE_TESTS_MODE) $(MAKE) test-live || { echo ""; echo "✗ CI failed at: test-live"; exit 1; }; \
	$(MAKE) build PROFILE=ci     || { echo ""; echo "✗ CI failed at: build"; exit 1; }; \
	$(MAKE) _lint-docs-check PROFILE=ci || { echo ""; echo "✗ CI failed at: lint-docs"; exit 1; }; \
	$(MAKE) examples-test        || { echo ""; echo "✗ CI failed at: examples-test"; exit 1; }
	@echo ""
	@echo "✓ Full CI completed successfully"

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
	@echo "  make format-check     - Verify rustfmt formatting"
	@echo "  make lint             - Clippy autofix + strict check + format check"
	@echo "  make lint-fix         - Clippy autofix only (mutating)"
	@echo "  make lint-check       - Clippy strict check + format check (non-mutating)"
	@echo "  make fix              - Apply formatting + clippy autofix"
	@echo "  make type-check       - Type check the workspace (cargo check)"
	@echo "  make clean            - Remove build artifacts (keeps Cargo.lock)"
	@echo "  make test             - Run all tests (workspace, libs/bins/tests only)"
	@echo "  make test-all         - Alias for make test"
	@echo "  make test-unit        - Run unit tests (lib and bins)"
	@echo "  make test-integration - Run integration tests"
	@echo "  make test-smoke       - Run deterministic smoke suite for PR validation"
	@echo "  make test-chaos       - Run chaos engineering tests"
	@echo "  make test-live        - Run live tests (LIVE_TESTS_MODE=required|optional|skip)"
	@echo "  make test-live-manual - Run manual live server test script"
	@echo "  make tui-smoke        - Run TUI UX smoke tests (snapshot suite, fast feedback)"
	@echo "  make tui-visual       - Run style-aware visual tests + interaction render checks"
	@echo "  make tui-accessibility - Run theme contrast/accessibility checks"
	@echo "  make run-tui          - Run TUI locally for manual testing"
	@echo "  make bench            - Run all benchmarks"
	@echo "  make bench-client     - Run client crate benchmarks"
	@echo "  make bench-cli        - Run CLI crate benchmarks"
	@echo "  make bench-tui        - Run TUI crate benchmarks"
	@echo "  make build            - Build workspace binaries (no install)"
	@echo "  make install-bins     - Install pre-built binaries to $(INSTALL_DIR)"
	@echo "  make release          - Build + install binaries to $(INSTALL_DIR)"
	@echo "  make generate         - Regenerate derived docs (cargo-run generate-tui-docs)"
	@echo "  make lint-docs        - Verify docs are up to date (cargo-run generate-tui-docs --check)"
	@echo "  make lint-secrets     - Run secret-commit guard"
	@echo "  make install-hooks    - Install git pre-commit hook for secret guard"
	@echo "  make ci-fast          - Fast local gate (non-mutating, smoke-focused)"
	@echo "  make ci               - Full local non-mutating pipeline"
	@echo "  knobs: CARGO_JOBS=<N> RUST_TEST_THREADS=<N> LIVE_TESTS_MODE=required|optional|skip CI_LIVE_TESTS_MODE=required|optional|skip"
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
