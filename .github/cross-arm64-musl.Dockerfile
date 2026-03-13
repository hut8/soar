# Custom cross-compilation image for aarch64-unknown-linux-musl
# Pre-installs all build dependencies to eliminate per-build apt-get overhead.
# Based on the official cross-rs image for this target.
#
# Build and publish via: .github/workflows/build-cross-arm64-image.yml
# Referenced by Cross.toml in PR #1216: [target.aarch64-unknown-linux-musl] image = "..."

FROM ghcr.io/cross-rs/aarch64-unknown-linux-musl:0.2.5

ENV DEBIAN_FRONTEND=noninteractive

# These are the same packages that were previously installed via Cross.toml pre-build
# commands on every single build. Baking them into the image saves ~1-2 minutes per build.
RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install -y -qq \
        build-essential \
        crossbuild-essential-arm64 \
        pkg-config \
        perl \
        clang \
        zlib1g-dev:arm64 \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
