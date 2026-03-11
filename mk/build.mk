# Purpose: Build, install, and release targets for supported runtime binaries.
# Responsibilities: Compile supported binaries without shipping internal tooling as runtime artifacts.
# Scope: Imported by the workspace root Makefile.
# Usage: Run via `make build`, `make install-bins`, or `make release`.
# Invariants/Assumptions: Only `splunk-cli` and `splunk-tui` are installable/runtime binaries.

build:
	@echo "→ Building binaries (profile: $(PROFILE))..."
	@$(CARGO_ENV) cargo build $(CARGO_JOBS_FLAG) --profile $(PROFILE) --package splunk-cli --bin splunk-cli --locked
	@$(CARGO_ENV) cargo build $(CARGO_JOBS_FLAG) --profile $(PROFILE) --package splunk-tui --bin splunk-tui --locked
	@echo "  ✓ Build complete"

install-bins:
	@mkdir -p $(INSTALL_DIR)
	@for bin in $(INSTALLABLE_BINS); do \
		echo "Installing $$bin to $(INSTALL_DIR)..."; \
		install -m 0755 target/$(TARGET_DIR)/$$bin $(INSTALL_DIR)/$$bin; \
	done
	@echo "  ✓ Binary install complete"

release: build install-bins
	@echo "  ✓ Release build + install complete"
