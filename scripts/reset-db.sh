#!/bin/bash
# Database Reset Wrapper Script

# Get the directory of this script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Run the Python script
exec python3 "$SCRIPT_DIR/reset-db.py" "$@"