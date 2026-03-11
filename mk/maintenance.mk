# Purpose: Shared maintenance and repository hygiene targets for local development.
# Responsibilities: Provide update, clean, type-check, secrets, and hook installation commands.
# Scope: Imported by the workspace root Makefile.
# Usage: Run via `make <target>` from the repository root.
# Invariants/Assumptions: Uses the root Makefile's shared variables and environment.

update:
	@echo "→ Upgrading direct dependency requirements..."
	@$(CARGO_ENV) cargo upgrade --incompatible
	@echo "→ Refreshing lockfile to latest allowed resolutions..."
	@$(CARGO_ENV) cargo update
	@echo "  ✓ Dependency update complete"

fix: format lint-fix

type-check:
	@echo "→ Type checking..."
	@$(CARGO_ENV) cargo check $(CARGO_JOBS_FLAG) --workspace --all-targets --all-features --locked
	@echo "  ✓ Type check complete"

lint-secrets:
	@bash scripts/check-secrets.sh

install-hooks:
	@ln -sf ../../scripts/check-secrets.sh .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Git pre-commit hook installed (pointing to scripts/check-secrets.sh)"

clean:
	@$(CARGO_ENV) cargo clean
	@rm -rf target/
