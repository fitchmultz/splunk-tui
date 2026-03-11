# Purpose: Test, smoke, chaos, benchmark, and local run targets for the Rust workspace.
# Responsibilities: Provide reproducible validation entrypoints that match the local-first gate.
# Scope: Imported by the workspace root Makefile.
# Usage: Run via `make <target>` from the repository root.
# Invariants/Assumptions: Test commands stay non-installing and respect the root Makefile knobs.

test: test-all

test-all:
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) --workspace --lib --bins --tests --all-features --locked -- $(TEST_ARGS)

test-unit:
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) --workspace --lib --bins --all-features --locked -- $(TEST_ARGS)

test-integration:
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) --workspace --tests --all-features --locked -- $(TEST_ARGS)

test-smoke:
	@echo "→ Running smoke test suite..."
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p architecture-tests --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-client --lib --all-features --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test auth_tests --test server_tests --test search_tests --all-features --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-cli --bins --all-features --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-cli --test health_tests --test search_tests --all-features --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-config --lib --all-features --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-config --test integration_test --all-features --locked -- $(TEST_ARGS)
	@$(MAKE) tui-smoke
	@$(MAKE) tui-visual
	@$(MAKE) tui-accessibility
	@echo "  ✓ Smoke tests complete"

test-chaos:
	@echo "→ Running chaos engineering tests..."
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test chaos_network_tests --features test-utils --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test chaos_timing_tests --features test-utils --locked -- $(TEST_ARGS)
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test chaos_flapping_tests --features test-utils --locked -- $(TEST_ARGS)
	@echo "  ✓ Chaos tests complete"

tui-smoke:
	@echo "→ Running TUI UX smoke tests..."
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-tui \
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

tui-visual:
	@echo "→ Running TUI visual style tests..."
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-tui \
		--test snapshot_styled_tests \
		--test interaction_render_tests \
		--all-features --locked -- $(TEST_ARGS)
	@echo "  ✓ TUI visual tests complete"

tui-accessibility:
	@echo "→ Running TUI accessibility contrast tests..."
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-tui \
		--test accessibility_contrast_tests \
		--all-features --locked -- $(TEST_ARGS)
	@echo "  ✓ TUI accessibility checks complete"

run-tui:
	@echo "→ Running TUI locally..."
	@$(CARGO_ENV) cargo run $(CARGO_JOBS_FLAG) --package splunk-tui --bin splunk-tui --all-features

test-live:
	@echo "→ Running live tests (mode: $(LIVE_TESTS_MODE))..."
	@mode="$(LIVE_TESTS_MODE)"; \
	case "$$mode" in \
		skip) \
			echo "  Skipping live tests (LIVE_TESTS_MODE=skip)"; \
			;; \
		*) \
			./scripts/validate-live-test-env.sh "$$mode"; \
			code=$$?; \
			if [ $$code -eq 2 ]; then \
				echo "  Live tests skipped (optional mode)"; \
			elif [ $$code -eq 1 ]; then \
				echo ""; \
				echo "✗ Live tests failed: environment not configured for required mode"; \
				exit 1; \
			else \
				echo "  Environment validated, running tests..."; \
				$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-client --test live_tests --all-features --locked -- --ignored $(TEST_ARGS) && \
				$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p splunk-cli --test live_tests --all-features --locked -- --ignored $(TEST_ARGS); \
			fi \
			;; \
	esac
	@echo "  ✓ Live tests complete"

test-live-manual:
	@bash scripts/test-live-server.sh

bench:
	@echo "→ Running all benchmarks..."
	@$(CARGO_ENV) cargo bench $(CARGO_JOBS_FLAG) --workspace

bench-client:
	@echo "→ Running client benchmarks..."
	@$(CARGO_ENV) cargo bench $(CARGO_JOBS_FLAG) -p splunk-client

bench-cli:
	@echo "→ Running CLI benchmarks..."
	@$(CARGO_ENV) cargo bench $(CARGO_JOBS_FLAG) -p splunk-cli

bench-tui:
	@echo "→ Running TUI benchmarks..."
	@$(CARGO_ENV) cargo bench $(CARGO_JOBS_FLAG) -p splunk-tui
