# Multi-stage build for SOAR
FROM node:20-alpine AS web-builder

# Install dependencies and build the web frontend
WORKDIR /app/web
COPY web/package*.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

# Rust build stage
FROM rust:1.75-slim-bullseye AS rust-builder

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

# Create a fake web directory structure for the build script
RUN mkdir -p web/node_modules

# Build the Rust application in release mode
RUN cargo build --release

# Upload debug symbols to Sentry (if SENTRY_AUTH_TOKEN is provided)
# This happens after build so we have the debug info files
# Using BuildKit secret mount for secure token handling
ARG SENTRY_ORG
ARG SENTRY_PROJECT
RUN --mount=type=secret,id=sentry_token \
    if [ -f /run/secrets/sentry_token ]; then \
      echo "Installing sentry-cli..." && \
      curl -sL https://sentry.io/get-cli/ | bash && \
      echo "Uploading debug symbols to Sentry..." && \
      export SENTRY_AUTH_TOKEN=$(cat /run/secrets/sentry_token) && \
      sentry-cli debug-files upload \
        --org "$SENTRY_ORG" \
        --project "$SENTRY_PROJECT" \
        target/release/soar || echo "Debug symbol upload failed (non-fatal)"; \
    else \
      echo "Skipping Sentry debug symbol upload (sentry_token secret not provided)"; \
    fi

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libpq5 \
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
