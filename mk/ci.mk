# Purpose: Local verification pipelines for fast and full repository gates.
# Responsibilities: Keep `make ci-fast` and `make ci` as the local-first source of truth.
# Scope: Imported by the workspace root Makefile.
# Usage: Run via `make ci-fast` or `make ci`.
# Invariants/Assumptions: CI stays local-only and non-installing except when users explicitly invoke install/release targets.

ci-fast:
	@echo "→ Local fast verification gate..."
	@echo ""
	@set -e; \
	$(MAKE) bootstrap             || { echo ""; echo "✗ CI (fast) failed at: bootstrap"; exit 1; }; \
	$(MAKE) deps                  || { echo ""; echo "✗ CI (fast) failed at: deps"; exit 1; }; \
	$(MAKE) format-check          || { echo ""; echo "✗ CI (fast) failed at: format-check"; exit 1; }; \
	$(MAKE) lint-secrets          || { echo ""; echo "✗ CI (fast) failed at: lint-secrets"; exit 1; }; \
	$(MAKE) lint-check            || { echo ""; echo "✗ CI (fast) failed at: lint-check"; exit 1; }; \
	$(MAKE) type-check            || { echo ""; echo "✗ CI (fast) failed at: type-check"; exit 1; }; \
	$(MAKE) test-smoke            || { echo ""; echo "✗ CI (fast) failed at: test-smoke"; exit 1; }; \
	$(MAKE) _lint-docs-check PROFILE=ci || { echo ""; echo "✗ CI (fast) failed at: lint-docs"; exit 1; }; \
	$(MAKE) examples-test         || { echo ""; echo "✗ CI (fast) failed at: examples-test"; exit 1; }
	@echo ""
	@echo "✓ Fast CI completed successfully"

ci:
	@echo "→ Local full CI (non-mutating, no install side effects)..."
	@echo ""
	@set -e; \
	$(MAKE) bootstrap             || { echo ""; echo "✗ CI failed at: bootstrap"; exit 1; }; \
	$(MAKE) deps                  || { echo ""; echo "✗ CI failed at: deps"; exit 1; }; \
	$(MAKE) format-check          || { echo ""; echo "✗ CI failed at: format-check"; exit 1; }; \
	$(MAKE) lint-secrets          || { echo ""; echo "✗ CI failed at: lint-secrets"; exit 1; }; \
	$(MAKE) lint-check            || { echo ""; echo "✗ CI failed at: lint-check"; exit 1; }; \
	$(MAKE) type-check            || { echo ""; echo "✗ CI failed at: type-check"; exit 1; }; \
	$(MAKE) test                  || { echo ""; echo "✗ CI failed at: test"; exit 1; }; \
	LIVE_TESTS_MODE=$(CI_LIVE_TESTS_MODE) $(MAKE) test-live || { echo ""; echo "✗ CI failed at: test-live"; exit 1; }; \
	$(MAKE) build PROFILE=ci      || { echo ""; echo "✗ CI failed at: build"; exit 1; }; \
	$(MAKE) _lint-docs-check PROFILE=ci || { echo ""; echo "✗ CI failed at: lint-docs"; exit 1; }; \
	$(MAKE) examples-test         || { echo ""; echo "✗ CI failed at: examples-test"; exit 1; }
	@echo ""
	@echo "✓ Full CI completed successfully"
