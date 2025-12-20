#!/bin/bash
#
# Dump raw APRS messages for a flight to a test data file and generate test case
#
# This script extracts all raw APRS messages associated with a flight,
# saves them to a test data file, and generates a new test case.
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
# Interactive:
#   The script will prompt you for a short description of the test case.
#   Example: "timeout resurrection creates new flight"
#
# Output:
#   - Test data file: tests/data/flights/[description].txt
#   - Test case: Appended to tests/flight_detection_test.rs

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

# Prompt for test case description
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Flight Detection Test Case Generator${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}Enter a short description for this test case:${NC}"
echo -e "${BLUE}Examples:${NC}"
echo -e "  - timeout resurrection creates new flight"
echo -e "  - missed landing detection"
echo -e "  - touch and go pattern"
echo -e "  - multiple thermals single flight"
echo ""
read -r -p "Description: " DESCRIPTION

# Validate description
if [ -z "$DESCRIPTION" ]; then
    echo -e "${RED}Error: Description cannot be empty${NC}"
    exit 1
fi

# Convert description to filename (slugify)
# - Convert to lowercase
# - Replace spaces with dashes
# - Remove special characters except dashes
# - Remove consecutive dashes
FILENAME=$(echo "$DESCRIPTION" | \
    tr '[:upper:]' '[:lower:]' | \
    sed 's/[^a-z0-9 -]//g' | \
    sed 's/ \+/-/g' | \
    sed 's/-\+/-/g' | \
    sed 's/^-//' | \
    sed 's/-$//')

# Validate filename
if [ -z "$FILENAME" ]; then
    echo -e "${RED}Error: Could not generate valid filename from description${NC}"
    exit 1
fi

# Define output files
OUTPUT_DIR="$GIT_ROOT/tests/data/flights"
OUTPUT_FILE="$OUTPUT_DIR/${FILENAME}.txt"
TEST_FILE="$GIT_ROOT/tests/flight_detection_test.rs"

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Dumping flight messages${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "Environment:  ${YELLOW}$ENVIRONMENT${NC}"
echo -e "Flight ID:    ${YELLOW}$FLIGHT_ID${NC}"
echo -e "Description:  ${YELLOW}$DESCRIPTION${NC}"
echo -e "Data file:    ${YELLOW}$OUTPUT_FILE${NC}"
echo -e "Test file:    ${YELLOW}$TEST_FILE${NC}"
echo ""

# Check if file already exists
if [ -f "$OUTPUT_FILE" ]; then
    echo -e "${YELLOW}Warning: File already exists: $OUTPUT_FILE${NC}"
    read -r -p "Overwrite? (y/N): " OVERWRITE
    if [ "$OVERWRITE" != "y" ] && [ "$OVERWRITE" != "Y" ]; then
        echo -e "${RED}Aborted${NC}"
        exit 1
    fi
fi

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

# Generate test function name from filename
TEST_FUNCTION_NAME="test_${FILENAME}"

# Generate test case code
echo -e "${YELLOW}Generating test case...${NC}"

# Create test case template
TEST_CASE=$(cat <<EOF

/// Test case: $DESCRIPTION
///
/// Flight ID: $FLIGHT_ID
/// Environment: $ENVIRONMENT
/// Messages: $MESSAGE_COUNT
/// Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
#[tokio::test]
#[ignore] // Remove this attribute once test is fully implemented
async fn ${TEST_FUNCTION_NAME}() {
    // ARRANGE: Set up test environment
    // TODO: Set up test database
    // let pool = setup_test_db().await;
    // let flight_tracker = FlightTracker::new(&pool);
    // let fix_processor = FixProcessor::new(...);

    // Load test messages from file
    let mut source = TestMessageSource::from_file("tests/data/flights/${FILENAME}.txt")
        .await
        .expect("Failed to load test messages");

    // ACT: Process all messages through the pipeline
    let mut messages_processed = 0;
    while let Some(message) = source.next_message().await.unwrap() {
        // TODO: Parse timestamp and APRS message
        // let (timestamp_str, aprs_message) = message.split_once(' ')
        //     .expect("Message should have timestamp");
        // let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
        //     .expect("Valid RFC3339 timestamp")
        //     .with_timezone(&Utc);

        // TODO: Process through the full pipeline
        // process_aprs_message(aprs_message, timestamp, &packet_router).await;

        messages_processed += 1;
    }

    // ASSERT: Verify expected behavior
    // TODO: Add assertions for expected flight detection behavior
    // Examples:
    // - Number of flights created
    // - Flight start/end times
    // - Flight state transitions
    // - Correct handling of timeouts, landings, etc.

    // Verify all messages were processed
    assert_eq!(messages_processed, ${MESSAGE_COUNT}, "Should process all messages");

    // TODO: Add specific assertions for this test case
    // Example:
    // let device_id = ...; // Extract from first message
    // let flights = get_flights_for_device(&pool, device_id).await;
    // assert_eq!(flights.len(), 2, "$DESCRIPTION");
}
EOF
)

# Append test case to test file
echo "$TEST_CASE" >> "$TEST_FILE"

echo -e "${GREEN}✓ Test case generated${NC}"
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Test case created successfully!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "Messages extracted: ${GREEN}$MESSAGE_COUNT${NC}"
echo -e "Data file:          ${GREEN}$OUTPUT_FILE${NC}"
echo -e "Test function:      ${GREEN}${TEST_FUNCTION_NAME}${NC}"
echo ""
echo -e "First few messages:"
echo -e "${BLUE}$(head -n 3 "$OUTPUT_FILE")${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Review the generated test in: $TEST_FILE"
echo "2. Implement the TODO sections (database setup, assertions)"
echo "3. Remove the #[ignore] attribute when ready to run"
echo "4. Run the test: cargo test ${TEST_FUNCTION_NAME}"
echo ""
