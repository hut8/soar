# Flight Detection Testing Framework

This document describes the testing framework for debugging and validating flight detection logic in SOAR.

## Overview

The flight detection testing framework allows you to:
- Extract real APRS message sequences from problematic flights
- Replay these messages in integration tests
- Verify that the flight detection logic behaves correctly
- Create regression tests for specific edge cases

This is particularly useful for debugging issues like:
- **Timeout resurrection**: Aircraft reappears after timeout (should it create a new flight or resume?)
- **Missed landing detection**: Aircraft descends and goes out of range while landing
- **Touch-and-go patterns**: Brief landings followed by immediate takeoff
- **Complex thermal patterns**: Multiple climbs and descents

## Quick Start Workflow

1. **Identify problematic flight** on Recent Flights page (copy flight UUID)
2. **Run script**: `./scripts/dump-flight-messages.sh production <flight-id>`
3. **Enter description** when prompted (e.g., "timeout resurrection creates new flight")
4. **Review generated test** in `tests/flight_detection_test.rs`
5. **Implement TODOs**: Add database setup, message processing, and assertions
6. **Remove `#[ignore]`** attribute and run the test

The script automatically:
- Extracts messages from the database
- Creates descriptively-named data file (e.g., `timeout-resurrection-creates-new-flight.txt`)
- Generates test case with arrange/act/assert structure
- Appends test to `tests/flight_detection_test.rs`

## Architecture

### Message Source Abstraction

The framework uses a trait-based abstraction (`RawMessageSource`) that allows the same processing code to consume messages from different sources:

```rust
#[async_trait]
pub trait RawMessageSource: Send + Sync {
    async fn next_message(&mut self) -> Result<Option<String>>;
    fn remaining_hint(&self) -> Option<usize> { None }
}
```

**Implementations:**
- **`NatsMessageSource`** - Production use (real-time streaming from NATS)
- **`TestMessageSource`** - Testing use (replay from files)

### Message Format

All messages follow the same format (both in NATS and test files):
```
YYYY-MM-DDTHH:MM:SS.SSSZ <raw_aprs_message>
```

**Example:**
```
2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNM:/074548h4821.86N/00531.07E'086/007/A=000607
```

Where:
- **Timestamp**: RFC3339 format, UTC timezone
- **Raw message**: Original APRS packet as received from APRS-IS

## Creating Test Cases

### Step 1: Identify a Problematic Flight

1. Navigate to the Recent Flights page in the SOAR web UI
2. Find a flight that exhibits the problematic behavior
3. Copy the flight UUID (visible in the URL or flight details)

### Step 2: Run the Test Case Generator

Use the `dump-flight-messages.sh` script to extract messages and automatically generate a test case:

```bash
# From staging database (local)
./scripts/dump-flight-messages.sh staging <flight-id>

# From production database (via SSH to glider.flights)
./scripts/dump-flight-messages.sh production <flight-id>
```

**Example:**
```bash
./scripts/dump-flight-messages.sh production 123e4567-e89b-12d3-a456-426614174000

# The script will prompt you:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Flight Detection Test Case Generator
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Enter a short description for this test case:
Examples:
  - timeout resurrection creates new flight
  - missed landing detection
  - touch and go pattern
  - multiple thermals single flight

Description: timeout resurrection creates new flight
```

**What the script does:**
1. Prompts for a descriptive name for the test case
2. Converts description to a filename (e.g., `timeout-resurrection-creates-new-flight.txt`)
3. Extracts all APRS messages from the database
4. Saves to `tests/data/flights/<descriptive-name>.txt`
5. Generates a test case with arrange/act/assert structure
6. Appends the test case to `tests/flight_detection_test.rs`

**Output:**
```
Messages extracted: 247
Data file:          tests/data/flights/timeout-resurrection-creates-new-flight.txt
Test function:      test_timeout_resurrection_creates_new_flight

Next steps:
1. Review the generated test in: tests/flight_detection_test.rs
2. Implement the TODO sections (database setup, assertions)
3. Remove the #[ignore] attribute when ready to run
4. Run the test: cargo test test_timeout_resurrection_creates_new_flight
```

### Step 3: Implement the Generated Test Case

The script generates a test template with TODO sections that you need to fill in:

The generated test follows the arrange/act/assert pattern:

```rust
/// Test case: timeout resurrection creates new flight
///
/// Flight ID: 123e4567-e89b-12d3-a456-426614174000
/// Environment: production
/// Messages: 247
/// Generated: 2025-12-20 22:15:30 UTC
#[tokio::test]
#[ignore] // Remove this attribute once test is fully implemented
async fn test_timeout_resurrection_creates_new_flight() {
    // ARRANGE: Set up test environment
    // TODO: Set up test database
    // let pool = setup_test_db().await;
    // let flight_tracker = FlightTracker::new(&pool);
    // let fix_processor = FixProcessor::new(...);

    // Load test messages from file
    let mut source = TestMessageSource::from_file(
        "tests/data/flights/timeout-resurrection-creates-new-flight.txt"
    ).await.expect("Failed to load test messages");

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
    assert_eq!(messages_processed, 247, "Should process all messages");

    // TODO: Add specific assertions for this test case
    // Example:
    // let device_id = ...; // Extract from first message
    // let flights = get_flights_for_device(&pool, device_id).await;
    // assert_eq!(flights.len(), 2, "timeout resurrection creates new flight");
}
```

**Steps to complete the test:**
1. Implement database setup (use existing test infrastructure or create new)
2. Add message parsing and processing logic
3. Add specific assertions for the expected behavior
4. Remove the `#[ignore]` attribute
5. Run the test: `cargo test test_timeout_resurrection_creates_new_flight`

## Using TestMessageSource

### Basic Usage

```rust
use soar::message_sources::{RawMessageSource, TestMessageSource};

// Create source from file
let mut source = TestMessageSource::from_file(
    "tests/data/flights/test-case.txt"
).await?;

// Process messages
while let Some(message) = source.next_message().await? {
    println!("Processing: {}", message);
}
```

### With Progress Tracking

```rust
// Create with known message count
let mut source = TestMessageSource::from_file_with_count(
    "tests/data/flights/test-case.txt",
    1000
).await?;

// Track progress
println!("Messages remaining: {:?}", source.remaining_hint());
println!("Messages read: {}", source.messages_read());
```

### Integration with Flight Detection Pipeline

```rust
// Create the full processing pipeline
let packet_router = PacketRouter::new(generic_processor)
    .with_aircraft_position_queue(aircraft_tx)
    // ... other queues
    .start(10);

// Process messages
while let Some(message) = source.next_message().await? {
    // Extract timestamp and APRS message
    let (timestamp_str, aprs_message) = message.split_once(' ')
        .expect("Message should have timestamp");

    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)?
        .with_timezone(&Utc);

    // Parse APRS message
    let parsed = ogn_parser::parse(aprs_message)?;

    // Process through the pipeline
    packet_router
        .process_packet(parsed, aprs_message, timestamp)
        .await;
}
```

## Test Scenarios to Cover

### 1. Timeout Resurrection
**Issue**: Aircraft reappears after timeout period (1 hour of inactivity)

**Test cases:**
- ✅ Should create NEW flight: Aircraft landed, timed out, then took off again
- ✅ Should RESUME existing flight: Brief loss of signal, aircraft still airborne

**Key distinguisher**: Was the aircraft descending/landing before timeout?

### 2. Landing Detection
**Issue**: Aircraft goes out of range while landing

**Test cases:**
- Gradual descent into landing pattern, last message shows low altitude and descent
- Sudden loss of signal while still at altitude (should keep flight open longer)

### 3. Touch-and-Go
**Issue**: Aircraft lands briefly then immediately takes off

**Test cases:**
- Should create one flight: Brief touchdown (< 5 minutes) with immediate takeoff
- Should create two flights: Extended ground time (> 10 minutes) then takeoff

### 4. Complex Thermal Patterns
**Issue**: Gliders often have complex climb/descent patterns

**Test cases:**
- Multiple thermal climbs with straight glides between them (should be one flight)
- Circuit pattern with multiple approaches (should be one flight until landed)

### 5. Multiple Aircraft
**Issue**: Ensure flight detection is independent per aircraft

**Test cases:**
- Process messages from multiple aircraft interleaved
- Verify flights are assigned to correct aircraft
- Ensure no cross-contamination of flight state

## Database Queries for Debugging

### Find Flights with Multiple Segments (Potential Resurrection Issues)

```sql
-- Find device IDs with multiple flights in a short time period
SELECT
    device_id,
    COUNT(*) as flight_count,
    MIN(started_at) as first_flight,
    MAX(started_at) as last_flight,
    MAX(started_at) - MIN(started_at) as time_span
FROM flights
WHERE started_at > NOW() - INTERVAL '7 days'
GROUP BY device_id
HAVING COUNT(*) > 5
ORDER BY flight_count DESC;
```

### Find Timed-Out Flights

```sql
-- Find flights that ended due to timeout
SELECT
    id,
    device_id,
    started_at,
    ended_at,
    end_reason,
    ended_at - started_at as duration
FROM flights
WHERE end_reason = 'timeout'
  AND ended_at > NOW() - INTERVAL '7 days'
ORDER BY ended_at DESC
LIMIT 100;
```

### Find Flights with Few Fixes (Potential Data Loss)

```sql
-- Find flights with unusually few fixes
SELECT
    f.id,
    f.device_id,
    f.started_at,
    f.ended_at,
    COUNT(fx.id) as fix_count,
    EXTRACT(EPOCH FROM (f.ended_at - f.started_at)) / 60 as duration_minutes
FROM flights f
LEFT JOIN fixes fx ON fx.flight_id = f.id
WHERE f.started_at > NOW() - INTERVAL '7 days'
GROUP BY f.id
HAVING COUNT(fx.id) < 10
ORDER BY fix_count ASC;
```

## Running Tests

### Run all flight detection tests

```bash
cargo test --test flight_detection_test
```

### Run specific test

```bash
cargo test --test flight_detection_test test_timeout_resurrection
```

### Run with output

```bash
cargo test --test flight_detection_test -- --nocapture
```

## Troubleshooting

### Script can't find git root
```bash
# Make sure you're in the git repository
cd /path/to/soar
./scripts/dump-flight-messages.sh staging <flight-id>
```

### No messages found for flight
- Verify the flight ID is correct (copy from URL or database)
- Check that the flight has associated fixes: `SELECT COUNT(*) FROM fixes WHERE flight_id = '<uuid>'`
- Ensure raw messages exist: Check `raw_message_id` is not NULL in fixes table

### SSH connection fails for production
```bash
# Verify SSH access to glider.flights
ssh glider.flights "psql -U postgres -d soar -c 'SELECT 1'"
```

### Test messages not parsing
- Verify file format: Each line should have timestamp, space, then APRS message
- Check for extra whitespace or encoding issues
- Use `head -n 5 tests/data/flights/file.txt` to inspect the file

## Best Practices

1. **Name test files descriptively**: Use scenario names instead of flight IDs
2. **Add comments to test files**: First line can be a comment explaining the scenario
3. **Keep test cases minimal**: Extract only the essential message sequence needed
4. **Version control test data**: Commit `.txt` files to git for reproducibility
5. **Document expected behavior**: Add comments in tests explaining why behavior is correct
6. **Test edge cases**: Focus on boundary conditions and known problem areas
7. **Use realistic data**: Real flights expose issues that synthetic data might miss

## Related Files

- **Message sources**: `src/message_sources.rs`
- **Dump script**: `scripts/dump-flight-messages.sh`
- **Test data**: `tests/data/flights/`
- **Example tests**: `tests/flight_detection_test.rs`
- **Flight tracker**: `src/flight_tracker/`
- **Fix processor**: `src/fix_processor.rs`

## Future Improvements

- [ ] Add `--filter` option to dump script (date range, message count limit)
- [ ] Create helper function for common test setup (database, processors, etc.)
- [ ] Add visualization tool to plot test flight trajectories
- [ ] Support for multi-flight test cases (interleaved aircraft)
- [ ] Automatic test case generation from production anomalies
- [ ] Performance benchmarking with test message replay
