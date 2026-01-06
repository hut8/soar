# Sentry Debug Symbol Upload

This document explains how debug symbols are generated and uploaded to Sentry for crash symbolication.

## Overview

Debug symbols allow Sentry to provide readable stack traces with file names and line numbers when crashes occur in production. Without debug symbols, you'll see raw memory addresses which are difficult to debug.

## Configuration

The project is configured to:
1. **Generate debug symbols** during release builds (in separate files)
2. **Strip symbols** from the release binary (for smaller size and security)
3. **Upload symbols** to Sentry separately

See `Cargo.toml`:
```toml
[profile.release]
debug = 2                    # Full debug info
strip = "symbols"            # Strip symbols from binary
split-debuginfo = "packed"   # Keep debug info in separate files
```

## Automatic Upload (CI/CD)

Debug symbols are automatically uploaded during Docker image builds in the release workflow.

### Required GitHub Secrets

Add these secrets to your GitHub repository:
1. Go to Settings → Secrets and variables → Actions
2. Add the following secrets:
   - `SENTRY_AUTH_TOKEN` - Auth token from Sentry (create at https://sentry.io/settings/account/api/auth-tokens/)
   - `SENTRY_ORG` - Your Sentry organization slug
   - `SENTRY_PROJECT` - Your Sentry project slug

When you push a version tag (e.g., `v1.0.0`), the release workflow will:
1. Build the release binary with debug info
2. Upload debug symbols to Sentry
3. Create a Docker image with the stripped binary

## Manual Upload

For local builds or manual deployments:

### Prerequisites

Install sentry-cli:
```bash
# macOS/Linux
curl -sL https://sentry.io/get-cli/ | bash

# Or via npm
npm install -g @sentry/cli

# Or via cargo
cargo install sentry-cli
```

### Upload Debug Symbols

```bash
# Set your auth token
export SENTRY_AUTH_TOKEN=your_token_here

# Option 1: Pass org and project as arguments
./scripts/upload-debug-symbols.sh your-org your-project

# Option 2: Set environment variables
export SENTRY_ORG=your-org
export SENTRY_PROJECT=your-project
./scripts/upload-debug-symbols.sh
```

## Verifying Uploads

After uploading:
1. Go to https://sentry.io/organizations/YOUR_ORG/projects/YOUR_PROJECT/settings/debug-symbols/
2. You should see your debug files listed
3. Check "Processing Status" - it should show "Processed"

## Testing

To test that symbolication works:
1. Deploy a version with uploaded debug symbols
2. Trigger a crash or error
3. Check Sentry - you should see readable stack traces with file names and line numbers

## Troubleshooting

### "Missing debug information file" error

This means:
- Debug symbols weren't uploaded for this binary, OR
- The binary was rebuilt and symbols don't match

Solution: Re-upload debug symbols for your current binary.

### Upload fails with "Permission denied"

Check that your `SENTRY_AUTH_TOKEN` has the correct permissions:
- project:read
- project:releases
- project:write

### Symbols uploaded but still seeing raw addresses

- Wait a few minutes for Sentry to process the symbols
- Verify the binary hash matches between what's running and what was uploaded
- Check that the version/release matches in Sentry

## References

- [Sentry Debug Files Documentation](https://docs.sentry.io/platforms/rust/data-management/debug-files/)
- [sentry-cli Documentation](https://docs.sentry.io/product/cli/)
- [Rust Debug Information](https://doc.rust-lang.org/cargo/reference/profiles.html#debug)
