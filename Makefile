# Purpose: Local automation and CI gate orchestration for the Splunk TUI Rust workspace.
# Responsibilities: Provide repeatable bootstrap, build, lint, test, release, docs, and container workflows.
# Scope: Local developer and workstation execution for the 5-crate workspace.
# Usage: Run `make help`; override knobs like `LIVE_TESTS_MODE`, `CARGO_JOBS`, and `RUST_TEST_THREADS` as needed.
# Invariants/Assumptions: Commands run from repo root with GNU Make and Cargo installed; the local-first gate remains the source of truth.

.PHONY: bootstrap deps install update lint lint-fix lint-check fix format format-check clean type-check \
	test test-all test-unit test-integration test-smoke test-chaos test-live test-live-manual \
	bench bench-client bench-cli bench-tui \
	build install-bins release generate lint-docs ci ci-fast help lint-secrets install-hooks \
	_generate-docs _lint-docs-check examples-test \
	tui-smoke tui-visual tui-accessibility run-tui \
	docker-build docker-run-cli docker-run-tui docker-compose-up \
	docker-compose-cli docker-compose-tui docker-clean

ROOT_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
MK_DIR := $(ROOT_DIR)mk

# Binaries and installation
INSTALLABLE_BINS := splunk-cli splunk-tui
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
SCCACHE_PATH := $(shell command -v sccache 2>/dev/null)
CARGO_ENV := $(if $(SCCACHE_PATH),RUSTC_WRAPPER=$(SCCACHE_PATH),)

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

# Bootstrap host prerequisites without mutating the repository.
bootstrap:
	@echo "→ Bootstrapping local workspace..."
	@command -v cargo >/dev/null || { echo "✗ cargo not found"; exit 1; }
	@command -v rustup >/dev/null || { echo "✗ rustup not found"; exit 1; }
	@echo "  Rust toolchain: $$(rustc --version)"
	@if [ -n "$(SCCACHE_PATH)" ]; then \
		echo "  sccache: enabled ($(SCCACHE_PATH))"; \
	else \
		echo "  sccache: not installed (builds will run normally)"; \
	fi
	@if command -v docker >/dev/null 2>&1; then \
		echo "  docker: available"; \
	else \
		echo "  docker: not installed (container workflows unavailable)"; \
	fi
	@if docker compose version >/dev/null 2>&1; then \
		echo "  docker compose: available"; \
	elif command -v docker-compose >/dev/null 2>&1; then \
		echo "  docker-compose: available"; \
	else \
		echo "  docker compose: not installed (compose workflows unavailable)"; \
	fi
	@echo "  ✓ Bootstrap complete"

# Fetch all dependencies (warm caches, no build)
deps:
	@echo "→ Fetching deps (locked)..."
	@$(CARGO_ENV) cargo fetch --locked
	@echo "  ✓ Deps fetched"

# Install supported runtime binaries after a build.
install: release
	@echo "  ✓ Installed supported binaries"

# Format code with rustfmt (write mode)
format:
	@echo "→ Formatting code..."
	@$(CARGO_ENV) cargo fmt --all
	@echo "  ✓ Formatting complete"

format-check:
	@echo "→ Format check..."
	@$(CARGO_ENV) cargo fmt --all --check
	@echo "  ✓ Format check complete"

# Run clippy (two-phase: autofix then strict check) and then verify formatting (cheap)
lint:
	@echo "→ Clippy autofix (phase 1/2)..."
	@$(CARGO_ENV) cargo clippy $(CARGO_JOBS_FLAG) --fix --allow-dirty --workspace --all-targets --all-features --locked
	@echo "→ Clippy strict check (phase 2/2)..."
	@$(CARGO_ENV) cargo clippy $(CARGO_JOBS_FLAG) --workspace --all-targets --all-features --locked -- -D warnings
	@echo "→ Format check..."
	@$(CARGO_ENV) cargo fmt --all --check
	@echo "  ✓ Lint complete"

lint-fix:
	@echo "→ Clippy autofix..."
	@$(CARGO_ENV) cargo clippy $(CARGO_JOBS_FLAG) --fix --allow-dirty --workspace --all-targets --all-features --locked
	@echo "  ✓ Clippy autofix complete"

lint-check:
	@echo "→ Clippy strict check..."
	@$(CARGO_ENV) cargo clippy $(CARGO_JOBS_FLAG) --workspace --all-targets --all-features --locked -- -D warnings
	@$(MAKE) format-check
	@echo "  ✓ Lint checks complete"

include $(MK_DIR)/maintenance.mk
include $(MK_DIR)/tests.mk
include $(MK_DIR)/build.mk
include $(MK_DIR)/docs.mk
include $(MK_DIR)/ci.mk
include $(MK_DIR)/docker.mk
include $(MK_DIR)/help.mk
