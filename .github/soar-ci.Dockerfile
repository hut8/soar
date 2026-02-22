# CI runner image for SOAR project
# Contains all build tools pre-installed to eliminate per-job setup time.
# Used with GitHub Actions `container:` directive.
#
# Build and publish via: .github/workflows/build-ci-image.yml
# Usage in CI: container: { image: ghcr.io/hut8/soar-ci:latest }

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# System packages (build tools, libs, utilities)
RUN apt-get update && apt-get install -y \
    git \
    curl \
    wget \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    musl-tools \
    protobuf-compiler \
    python3 \
    jq \
    file \
    && rm -rf /var/lib/apt/lists/*

# PostgreSQL 17 client
RUN install -d /usr/share/postgresql-common/pgdg && \
    curl -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc --fail \
      https://www.postgresql.org/media/keys/ACCC4CF8.asc && \
    echo "deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt jammy-pgdg main" \
      > /etc/apt/sources.list.d/pgdg.list && \
    apt-get update && \
    apt-get install -y postgresql-client-17 && \
    rm -rf /var/lib/apt/lists/*

# Node.js 24
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Playwright: install system deps and Chromium browser binary.
# Pin version to match web/package.json @playwright/test version.
# When bumping Playwright in package.json, update this version too.
ENV PLAYWRIGHT_VERSION=1.58.2
RUN npx playwright@${PLAYWRIGHT_VERSION} install --with-deps chromium

# Rust toolchain (stable with clippy, rustfmt, musl target)
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/usr/local/cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain stable \
    --component clippy,rustfmt \
    --target x86_64-unknown-linux-musl

# Cargo tools
RUN cargo install diesel_cli --no-default-features --features postgres --version 2.2.0 && \
    cargo install cargo-nextest --version 0.9.68 --locked && \
    cargo install diesel-guard --version 0.5.0 && \
    cargo install cargo-audit && \
    cargo install cargo-outdated && \
    rm -rf /usr/local/cargo/registry /usr/local/cargo/git

# Mark all directories as safe for git (needed for actions/checkout in containers)
RUN git config --global --add safe.directory '*'

# Disable incremental compilation for CI (standard practice)
ENV CARGO_INCREMENTAL=0
