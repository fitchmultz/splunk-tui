# Purpose: Container build and local runtime targets for supported Docker workflows.
# Responsibilities: Build the runtime image and run CLI/TUI or docker-compose flows without any removed orchestration surface.
# Scope: Imported by the workspace root Makefile.
# Usage: Run via `make docker-build`, `make docker-run-cli`, `make docker-run-tui`, or compose targets.
# Invariants/Assumptions: Docker/docker-compose are optional local dependencies; unsupported orchestration targets remain out of scope.

docker-build:
	@echo "→ Building Docker image..."
	@docker build -t splunk-tui:latest .
	@echo "  ✓ Docker image built (splunk-tui:latest)"

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

docker-compose-up:
	@echo "→ Starting docker-compose services..."
	@docker-compose up -d splunk
	@echo "  ✓ Services started"
	@echo ""
	@echo "  Splunk is starting up. Health check will run automatically."
	@echo "  Web UI: http://localhost:8000"
	@echo "  REST API: https://localhost:8089"

docker-compose-cli:
	@echo "→ Running CLI via docker-compose..."
	@docker-compose --profile cli run --rm cli $(ARGS)
	@echo "  ✓ CLI service exited"

docker-compose-tui:
	@echo "→ Running TUI via docker-compose..."
	@docker-compose --profile tui run --rm tui $(ARGS)
	@echo "  ✓ TUI service exited"

docker-clean:
	@echo "→ Removing local Docker images..."
	@docker rmi splunk-tui:latest 2>/dev/null || echo "  Image not found or already removed"
	@docker-compose down --rmi local 2>/dev/null || true
	@echo "  ✓ Docker cleanup complete"
