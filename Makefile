.PHONY: install update lint type-check format clean test build ci help

# Default target
.DEFAULT_GOAL := help

# Install all dependencies and binaries to ~/.cargo/bin
install:
	cargo install --path crates/cli --force
	cargo install --path crates/tui --force

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
test:
	cargo test --all-features

# Release build
build:
	cargo build --release --all-features

# CI pipeline: format -> lint -> type-check -> test -> build
ci: format lint type-check test build

# Display help for each target
help:
	@echo "Splunk TUI - Available targets:"
	@echo ""
	@echo "  make install      - Install binaries to ~/.cargo/bin"
	@echo "  make update       - Update all dependencies to latest stable versions"
	@echo "  make lint         - Run clippy (warnings as errors)"
	@echo "  make type-check   - Run cargo check"
	@echo "  make format       - Format code with rustfmt (write mode)"
	@echo "  make clean        - Remove build artifacts and lock files"
	@echo "  make test         - Run all tests"
	@echo "  make build        - Release build"
	@echo "  make ci           - Run full CI pipeline (format -> lint -> type-check -> test -> build)"
	@echo "  make help         - Show this help message"
