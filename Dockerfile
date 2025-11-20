# Multi-stage build for minimal production image
# Stage 1: Build the Rust binary
FROM rust:1.70-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual binary
# Touch main.rs to ensure it's rebuilt
RUN touch src/main.rs && \
    cargo build --release --bin with-env

# Stage 2: Create minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 -s /bin/bash appuser

# Copy binary from builder
COPY --from=builder /app/target/release/with-env /usr/local/bin/with-env

# Create config directory and set permissions
RUN mkdir -p /home/appuser/.config/with-env/envs && \
    chown -R appuser:appuser /home/appuser/.config

# Switch to non-root user
USER appuser
WORKDIR /home/appuser

# Environment variables (to be set at runtime)
ENV GITHUB_TOKEN=""
ENV ORGANIZATION=""

# Health check (optional)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD with-env --version || exit 1

ENTRYPOINT ["with-env"]
CMD ["--help"]
