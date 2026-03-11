# Purpose: Documentation generation and drift-check targets.
# Responsibilities: Regenerate and verify derived docs using the internal docs generator only.
# Scope: Imported by the workspace root Makefile.
# Usage: Run via `make generate` or `make lint-docs`.
# Invariants/Assumptions: `generate-tui-docs` is internal tooling and is never installed as a runtime binary.

_generate-docs:
	@echo "→ Generating derived docs..."
	@$(CARGO_ENV) cargo run $(CARGO_JOBS_FLAG) --profile $(PROFILE) --package splunk-tui --bin generate-tui-docs --locked --
	@echo "  ✓ Generated"

_lint-docs-check:
	@echo "→ Checking docs drift..."
	@$(CARGO_ENV) cargo run $(CARGO_JOBS_FLAG) --profile $(PROFILE) --package splunk-tui --bin generate-tui-docs --locked -- --check
	@echo "  ✓ Docs clean"

generate: _generate-docs

lint-docs: _lint-docs-check

examples-test:
	@echo "→ Validating example scripts..."
	@find examples -name "*.sh" -type f | while read script; do \
		echo "  Checking $$script..."; \
		bash -n "$$script" || { echo ""; echo "✗ Syntax error in $$script"; exit 1; }; \
		[ -x "$$script" ] || { echo ""; echo "✗ Not executable: $$script"; exit 1; }; \
	done
	@echo "→ Checking example/docs CLI contract drift..."
	@$(CARGO_ENV) cargo test $(CARGO_JOBS_FLAG) -p architecture-tests --test example_cli_contract_tests --locked -- --test-threads 1
	@echo "  ✓ All example scripts validated"
