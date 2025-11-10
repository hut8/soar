# Multi-stage build for SOAR
FROM node:20-alpine AS web-builder

# Install dependencies and build the web frontend
WORKDIR /app/web
COPY web/package*.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

# Rust build stage
FROM rust:1-slim-bookworm AS rust-builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Rust configuration files
COPY Cargo.toml Cargo.lock build.rs ./
COPY src/ ./src/
COPY migrations/ ./migrations/

# Copy the built web assets from previous stage
COPY --from=web-builder /app/web/build/ ./web/build/
COPY --from=web-builder /app/web/package*.json ./web/

# Build the Rust application in release mode
# Set SKIP_WEB_BUILD because web is already built in the web-builder stage
RUN SKIP_WEB_BUILD=1 cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies including debug symbols for Sentry symbolication
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    libc6-dbg \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN groupadd -r soar && useradd -r -g soar soar

# Create app directory
WORKDIR /app

# Copy the built binary
COPY --from=rust-builder /app/target/release/soar /usr/local/bin/soar

# Make sure the binary is executable
RUN chmod +x /usr/local/bin/soar

# Change ownership to the soar user
RUN chown soar:soar /app

# Switch to non-root user
USER soar

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
  CMD soar --help > /dev/null || exit 1

# Default command
CMD ["soar", "--help"]

# Expose default web server port
EXPOSE 61225

# Add labels for metadata
LABEL org.opencontainers.image.title="SOAR"
LABEL org.opencontainers.image.description="SOAR - Soaring Observation And Records"
LABEL org.opencontainers.image.vendor="SOAR Project"
LABEL org.opencontainers.image.licenses="MIT"
