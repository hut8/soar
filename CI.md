# CI/CD Documentation

This document describes the Continuous Integration and Continuous Deployment setup for the SOAR project.

## Overview

The SOAR project uses GitHub Actions for CI/CD with the following workflows:

1. **CI Workflow** (`.github/workflows/ci.yml`) - Runs on pushes and pull requests
2. **Release Workflow** (`.github/workflows/release.yml`) - Runs on git tags

## CI Workflow

The CI workflow consists of several jobs that run in sequence:

### 1. Test SvelteKit Project (`test-sveltekit`)

- **Platform**: Ubuntu Latest
- **Node.js**: Version 20
- **Steps**:
  - Install dependencies with `npm ci`
  - Run linter with `npm run lint`
  - Run type checking with `npm run check`
  - Run tests with `npm test`
  - Build the project with `npm run build`
  - Upload build artifacts for use by Rust build

### 2. Test Rust Project (`test-rust`)

- **Platform**: Ubuntu Latest
- **Database**: PostgreSQL 15 with PostGIS extension
- **Dependencies**: Requires SvelteKit build artifacts
- **Steps**:
  - Download web build artifacts
  - Install Rust toolchain with clippy and rustfmt
  - Check code formatting with `cargo fmt --check`
  - Run Clippy linter with `cargo clippy`
  - Run Rust tests with `cargo test`
  - Verify database migrations work

### 3. Build Release Binary (`build-release`)

- **Platform**: Ubuntu Latest
- **Dependencies**: Requires both SvelteKit and Rust tests to pass
- **Steps**:
  - Download web build artifacts
  - Build release binary with `cargo build --release`
  - Create compressed archive
  - Upload binary artifact
  - Display binary information

### 4. Security Audit (`security-audit`)

- **Platform**: Ubuntu Latest
- **Steps**:
  - Run `cargo audit` for security vulnerabilities
  - Check for outdated dependencies with `cargo outdated`

## Release Workflow

The release workflow is triggered when a git tag starting with 'v' is pushed (e.g., `v1.0.0`).

### 1. Create Release (`create-release`)

- Creates a GitHub release with release notes
- Extracts version from git tag
- Provides upload URL for release assets

### 2. Build Release Matrix (`build-release-matrix`)

Builds release binaries for multiple platforms:

- **Linux x64** (GNU libc)
- **Linux x64** (musl libc)
- **macOS x64** (Intel)
- **macOS ARM64** (Apple Silicon)
- **Windows x64**

Each platform:
- Builds both web and Rust components
- Creates platform-specific archives (.tar.gz or .zip)
- Uploads as release assets

### 3. Build Docker Image (`build-docker`)

- Builds multi-stage Docker image
- Pushes to GitHub Container Registry
- Tags with both version and 'latest'
- Uses build cache for optimization

## Docker Support

The project includes a multi-stage Dockerfile:

1. **Web Builder Stage**: Builds SvelteKit frontend
2. **Rust Builder Stage**: Builds Rust binary with embedded web assets
3. **Runtime Stage**: Minimal Debian image with just the binary

Key features:
- Runs as non-root user
- Includes health check
- Exposes port 61225 by default
- Optimized for size and security

## Environment Variables

### CI Environment

- `CARGO_TERM_COLOR=always`: Colored Rust output
- `RUST_BACKTRACE=1`: Detailed error traces
- `DATABASE_URL`: PostgreSQL connection string for tests

### Database Setup

The CI automatically sets up PostgreSQL with:
- Version 15
- PostGIS extension enabled
- Test database `soar_test`
- User/password: `postgres/postgres`

## Running Locally

To run the same checks locally:

```bash
# SvelteKit checks
cd web
npm ci
npm run lint
npm run check
npm test
npm run build

# Rust checks
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --verbose

# Build release binary
cargo build --release

# Security audit
cargo install cargo-audit
cargo audit
```

## Docker Build Locally

```bash
# Build the Docker image
docker build -t soar:local .

# Run the container
docker run --rm -p 61225:61225 soar:local soar web

# Or run with help
docker run --rm soar:local soar --help
```

## Troubleshooting

### Common Issues

1. **Web build artifacts missing**: The Rust build depends on SvelteKit artifacts
2. **Database connection failed**: Check PostgreSQL service status in CI
3. **Binary not found**: Ensure both stages completed successfully
4. **Permission denied**: Check Docker user permissions and file ownership

### Debugging CI

- Check the "Actions" tab in GitHub for detailed logs
- Each job shows step-by-step execution
- Artifacts are preserved for 30 days for debugging
- Failed jobs can be re-run individually
