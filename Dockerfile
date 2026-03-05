# syntax=docker/dockerfile:1

# Stage 1: Chef - Base image with cargo-chef for dependency caching
FROM lukemathwalker/cargo-chef:latest-rust-1.84 AS chef
WORKDIR /app

# Stage 2: Planner - Generate recipe.json for dependency caching
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Compile the application
FROM chef AS builder

# Install dependencies for building
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy recipe and build dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY . .
RUN cargo build --release --workspace --bins --locked

# Stage 3b: Build minimal pause binary for Kubernetes deployments
# This is needed because distroless images have no shell or sleep command
FROM rust:1.84-slim AS pause-builder
RUN echo 'fn main() { loop { std::thread::sleep(std::time::Duration::from_secs(3600)); } }' > /tmp/pause.rs && \
    rustc /tmp/pause.rs -o /tmp/pause

# Stage 4: Runtime - Minimal distroless image
FROM gcr.io/distroless/cc-debian12:nonroot

# Copy CA certificates for HTTPS connections to Splunk API
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy application binaries from builder
COPY --from=builder --chown=nonroot:nonroot /app/target/release/splunk-cli /usr/local/bin/splunk-cli
COPY --from=builder --chown=nonroot:nonroot /app/target/release/splunk-tui /usr/local/bin/splunk-tui

# Copy pause binary for Kubernetes deployments (keeps container alive)
COPY --from=pause-builder --chown=nonroot:nonroot /tmp/pause /usr/local/bin/pause

# Use non-root user (distroless nonroot user is UID 65532)
USER nonroot

# Default to CLI (TUI requires TTY which docker run -it provides)
ENTRYPOINT ["/usr/local/bin/splunk-cli"]
