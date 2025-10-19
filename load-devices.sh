#!/bin/bash
# Load devices from the unified FlarmNet database
# Download the latest unified FlarmNet database from:
# https://turbo87.github.io/united-flarmnet/united.fln

set -e

DEVICES_FILE="${1:-united.fln}"

if [ ! -f "$DEVICES_FILE" ]; then
    echo "Error: Devices file not found: $DEVICES_FILE"
    echo ""
    echo "Usage: $0 [path/to/united.fln]"
    echo ""
    echo "Download the unified FlarmNet database from:"
    echo "  https://turbo87.github.io/united-flarmnet/united.fln"
    echo ""
    echo "Example:"
    echo "  curl -O https://turbo87.github.io/united-flarmnet/united.fln"
    echo "  $0 united.fln"
    exit 1
fi

echo "Loading devices from: $DEVICES_FILE"
cargo run --release -- load-data --devices "$DEVICES_FILE"
