#!/bin/bash
#
# Dump raw APRS messages for a flight to a test data file
#
# This script extracts all raw APRS messages associated with a flight
# and saves them in a format suitable for integration testing.
#
# Usage:
#   ./scripts/dump-flight-messages.sh <environment> <flight_id>
#
# Arguments:
#   environment - Either "staging" or "production"
#   flight_id   - UUID of the flight to dump
#
# Examples:
#   ./scripts/dump-flight-messages.sh staging 123e4567-e89b-12d3-a456-426614174000
#   ./scripts/dump-flight-messages.sh production 123e4567-e89b-12d3-a456-426614174000
#
# Output:
#   Creates file at tests/data/flights/[flight_id]-ogn-aprs.txt
#   Format: Each line contains "YYYY-MM-DDTHH:MM:SS.SSSZ <raw_aprs_message>"

set -e  # Exit on error

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Validate arguments
if [ "$#" -ne 2 ]; then
    echo -e "${RED}Error: Invalid number of arguments${NC}"
    echo ""
    echo "Usage: $0 <environment> <flight_id>"
    echo ""
    echo "Arguments:"
    echo "  environment  Either 'staging' or 'production'"
    echo "  flight_id    UUID of the flight to dump"
    echo ""
    echo "Examples:"
    echo "  $0 staging 123e4567-e89b-12d3-a456-426614174000"
    echo "  $0 production 123e4567-e89b-12d3-a456-426614174000"
    exit 1
fi

ENVIRONMENT="$1"
FLIGHT_ID="$2"

# Validate environment
if [ "$ENVIRONMENT" != "staging" ] && [ "$ENVIRONMENT" != "production" ]; then
    echo -e "${RED}Error: Environment must be 'staging' or 'production'${NC}"
    exit 1
fi

# Validate flight_id format (basic UUID check)
if ! echo "$FLIGHT_ID" | grep -qE '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'; then
    echo -e "${RED}Error: Invalid flight_id format. Must be a valid UUID.${NC}"
    exit 1
fi

# Find git root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GIT_ROOT="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel 2>/dev/null)" || {
    echo -e "${RED}Error: Not in a git repository${NC}"
    exit 1
}

# Define output directory and file
OUTPUT_DIR="$GIT_ROOT/tests/data/flights"
OUTPUT_FILE="$OUTPUT_DIR/${FLIGHT_ID}-ogn-aprs.txt"

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Dumping flight messages${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "Environment: ${YELLOW}$ENVIRONMENT${NC}"
echo -e "Flight ID:   ${YELLOW}$FLIGHT_ID${NC}"
echo -e "Output:      ${YELLOW}$OUTPUT_FILE${NC}"
echo ""

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"
echo -e "${GREEN}✓ Output directory ready${NC}"

# SQL query to extract raw messages
SQL_QUERY="
SELECT
    TO_CHAR(rm.received_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') AS timestamp,
    encode(rm.raw_message, 'escape') AS raw_message
FROM raw_messages rm
JOIN fixes f ON f.raw_message_id = rm.id
WHERE f.flight_id = '$FLIGHT_ID'
  AND rm.source = 'aprs'
ORDER BY rm.received_at ASC;
"

# Function to run query locally (staging)
run_query_local() {
    local db_name="$1"
    echo -e "${YELLOW}Querying local database: $db_name${NC}"

    psql -U postgres -d "$db_name" -t -A -F ' ' -c "$SQL_QUERY" > "$OUTPUT_FILE" || {
        echo -e "${RED}Failed to query database${NC}"
        exit 1
    }
}

# Function to run query remotely (production)
run_query_remote() {
    echo -e "${YELLOW}Querying remote database via SSH: glider.flights${NC}"

    ssh glider.flights "psql -U postgres -d soar -t -A -F ' ' -c \"$SQL_QUERY\"" > "$OUTPUT_FILE" || {
        echo -e "${RED}Failed to query remote database${NC}"
        exit 1
    }
}

# Execute query based on environment
if [ "$ENVIRONMENT" = "staging" ]; then
    run_query_local "soar_dev"
else
    run_query_remote
fi

# Check if output file has content
if [ ! -s "$OUTPUT_FILE" ]; then
    echo -e "${RED}Error: No messages found for flight $FLIGHT_ID${NC}"
    echo -e "${YELLOW}The flight may not exist or has no associated raw messages${NC}"
    rm -f "$OUTPUT_FILE"
    exit 1
fi

# Count messages
MESSAGE_COUNT=$(wc -l < "$OUTPUT_FILE")

echo -e "${GREEN}✓ Query completed${NC}"
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Flight messages dumped successfully!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "Messages extracted: ${GREEN}$MESSAGE_COUNT${NC}"
echo -e "Output file:        ${GREEN}$OUTPUT_FILE${NC}"
echo ""
echo -e "First few messages:"
echo -e "${BLUE}$(head -n 3 "$OUTPUT_FILE")${NC}"
echo ""
echo "You can now use this file in integration tests with TestMessageSource."
