#!/bin/bash
set -e

# Upload debug symbols to Sentry
# This script uploads debug symbols from a release build to Sentry for crash symbolication
#
# Usage:
#   ./scripts/upload-debug-symbols.sh
#
# Requirements:
#   - sentry-cli installed (https://docs.sentry.io/product/cli/installation/)
#   - SENTRY_AUTH_TOKEN environment variable set
#   - SENTRY_ORG environment variable set (or passed as argument)
#   - SENTRY_PROJECT environment variable set (or passed as argument)
#
# Environment variables:
#   SENTRY_AUTH_TOKEN - Sentry authentication token
#   SENTRY_ORG - Sentry organization slug (default: from .sentryclirc or CLI arg)
#   SENTRY_PROJECT - Sentry project slug (default: from .sentryclirc or CLI arg)

# Check if sentry-cli is installed
if ! command -v sentry-cli &> /dev/null; then
    echo "Error: sentry-cli is not installed"
    echo "Install it from: https://docs.sentry.io/product/cli/installation/"
    echo ""
    echo "Quick install:"
    echo "  curl -sL https://sentry.io/get-cli/ | bash"
    exit 1
fi

# Check if SENTRY_AUTH_TOKEN is set
if [ -z "$SENTRY_AUTH_TOKEN" ]; then
    echo "Error: SENTRY_AUTH_TOKEN environment variable is not set"
    echo "Get your auth token from: https://sentry.io/settings/account/api/auth-tokens/"
    exit 1
fi

# Set org and project from arguments or environment
ORG="${1:-$SENTRY_ORG}"
PROJECT="${2:-$SENTRY_PROJECT}"

if [ -z "$ORG" ]; then
    echo "Error: Sentry organization not specified"
    echo "Usage: $0 [org] [project]"
    echo "Or set SENTRY_ORG environment variable"
    exit 1
fi

if [ -z "$PROJECT" ]; then
    echo "Error: Sentry project not specified"
    echo "Usage: $0 [org] [project]"
    echo "Or set SENTRY_PROJECT environment variable"
    exit 1
fi

# Check if release binary exists
if [ ! -f "target/release/soar" ]; then
    echo "Error: Release binary not found at target/release/soar"
    echo "Build it first with: cargo build --release"
    exit 1
fi

echo "Uploading debug symbols to Sentry..."
echo "  Organization: $ORG"
echo "  Project: $PROJECT"
echo "  Binary: target/release/soar"
echo ""

# Upload debug symbols
sentry-cli debug-files upload \
    --org "$ORG" \
    --project "$PROJECT" \
    target/release/soar

echo ""
echo "✓ Debug symbols uploaded successfully!"
echo ""
echo "Note: It may take a few minutes for Sentry to process the symbols."
echo "Check processing status at: https://sentry.io/organizations/$ORG/projects/$PROJECT/settings/debug-symbols/"
