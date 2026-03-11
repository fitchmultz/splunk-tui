# syntax=docker/dockerfile:1

# Stage 1: Chef - Base image with cargo-chef for dependency caching
FROM rust:1.94.0-slim AS chef
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* && \
    cargo install cargo-chef --locked

# Stage 2: Planner - Generate recipe.json for dependency caching
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Compile the application
FROM chef AS builder

# Copy recipe and build dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY . .
RUN cargo build --release --package splunk-cli --bin splunk-cli --locked && \
    cargo build --release --package splunk-tui --bin splunk-tui --locked

# Stage 4: Runtime - Minimal distroless image
FROM gcr.io/distroless/cc-debian12:nonroot

# Copy CA certificates for HTTPS connections to Splunk API
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy application binaries from builder
COPY --from=builder --chown=nonroot:nonroot /app/target/release/splunk-cli /usr/local/bin/splunk-cli
COPY --from=builder --chown=nonroot:nonroot /app/target/release/splunk-tui /usr/local/bin/splunk-tui

# Use non-root user (distroless nonroot user is UID 65532)
USER nonroot

# Default to CLI (TUI requires TTY which docker run -it provides)
ENTRYPOINT ["/usr/local/bin/splunk-cli"]
