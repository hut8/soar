# Static Linking with musl for Portable Binaries

## Overview

This document explains the changes made to create fully static, portable binaries for the SOAR project.

## Problem

The original build used `x86_64-unknown-linux-gnu` target which creates binaries that dynamically link against:
- `glibc` (GNU C Library)
- `libgdal` (GDAL geospatial library)
- `libpq` (PostgreSQL client library)
- `libssl` / `libcrypto` (OpenSSL)
- Other system libraries

This causes deployment issues:
- Version mismatches between build and deployment servers
- Missing libraries on destination servers
- Complex dependency management during deployment

## Solution

Use **musl libc** instead of glibc, which enables full static linking:
- Target: `x86_64-unknown-linux-musl` (and `aarch64-unknown-linux-musl` for ARM64)
- Build tool: `cross` with Alpine Linux-based Docker images
- Result: Single binary with no external dependencies

## Changes Made

### 1. Cross.toml
Added musl target configurations:
```toml
[target.x86_64-unknown-linux-musl]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-musl:edge"
pre-build = [
    "apk add --no-cache gdal-dev gdal-static postgresql-dev pkgconfig",
]

[target.aarch64-unknown-linux-musl]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-musl:edge"
pre-build = [
    "apk add --no-cache gdal-dev gdal-static postgresql-dev pkgconfig",
]
```

### 2. .cargo/config.toml
Added rustflags for static linking:
```toml
[target.x86_64-unknown-linux-musl]
rustflags = [
    "--cfg", "tokio_unstable",
    "-C", "target-feature=+crt-static",
    "-C", "link-arg=-static",
]
```

### 3. GitHub Actions Workflows

#### ci.yml
- Changed `build-release` job to use `x86_64-unknown-linux-musl`
- Changed `build-release-arm64` to use `aarch64-unknown-linux-musl`
- Removed GDAL/mold installation steps (handled by cross)
- Added `cross` installation and caching
- Added static linking verification

#### release.yml (TODO)
- Same changes as ci.yml for release builds

#### deploy-adsb.yml (TODO)
- Same changes as ci.yml for ADS-B deployment builds

## Benefits

1. **Portability**: Binary runs on any Linux system (kernel 2.6.32+)
2. **Simplicity**: No need to install dependencies on deployment servers
3. **Consistency**: Same binary works across different distros/versions
4. **Security**: Easier to audit - all dependencies bundled
5. **Size**: Similar size to dynamically linked builds

## Verification

The workflow includes a verification step:
```bash
ldd target/x86_64-unknown-linux-musl/release/soar
# Should output: "not a dynamic executable"
```

## Testing

1. Build binary with musl: `cross build --release --target x86_64-unknown-linux-musl`
2. Verify static linking: `ldd target/x86_64-unknown-linux-musl/release/soar`
3. Test on different Linux distros (Ubuntu, Debian, Alpine, CentOS, etc.)

## Trade-offs

**Pros:**
- Complete portability
- No dependency hell
- Simplified deployment

**Cons:**
- Slightly slower DNS resolution (musl's getaddrinfo)
- Larger build time (static linking all dependencies)
- Cannot use glibc-specific features

## Next Steps

- [ ] Finish updating release.yml
- [ ] Finish updating deploy-adsb.yml
- [ ] Test binary on multiple Linux distributions
- [ ] Update deployment scripts to remove dependency installation steps
- [ ] Document deployment process
