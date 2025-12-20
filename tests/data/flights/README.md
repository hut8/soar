# Flight Test Data

This directory contains raw APRS message dumps from real flights, used for integration testing of the flight detection logic.

## File Format

Each file contains raw APRS messages in chronological order, with descriptive filenames based on the test scenario.

**Format:** Each line contains a timestamp and raw APRS message separated by a single space:
```
2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNM:/074548h4821.86N/00531.07E'086/007/A=000607 !W84! id06DDA5BA -019fpm +0.0rot 5.5dB 0e -4.3kHz
```

**Fields:**
- **Timestamp**: ISO 8601 format with UTC timezone (`YYYY-MM-DDTHH:MM:SS.SSSZ`)
- **Raw Message**: Original APRS packet as received from APRS-IS

## Generating Test Cases

The `dump-flight-messages.sh` script automatically:
1. Extracts messages from the database
2. Generates a descriptive filename
3. Creates a corresponding test case with arrange/act/assert structure

### Usage

```bash
# Run the script with environment and flight ID
./scripts/dump-flight-messages.sh production <flight-id>

# The script will prompt you for a description
Description: timeout resurrection creates new flight

# Output:
# - Data file: tests/data/flights/timeout-resurrection-creates-new-flight.txt
# - Test case: Added to tests/flight_detection_test.rs
```

### Example Descriptions

- `timeout resurrection creates new flight`
- `missed landing detection`
- `touch and go pattern`
- `multiple thermals single flight`
- `glider tow release detection`

The script will convert these to valid filenames:
- `timeout-resurrection-creates-new-flight.txt`
- `missed-landing-detection.txt`
- `touch-and-go-pattern.txt`
- etc.

## Generated Test Cases

Each test case follows the arrange/act/assert pattern:

```rust
#[tokio::test]
#[ignore] // Remove when ready to implement
async fn test_timeout_resurrection_creates_new_flight() {
    // ARRANGE: Set up test environment
    let mut source = TestMessageSource::from_file(
        "tests/data/flights/timeout-resurrection-creates-new-flight.txt"
    ).await.expect("Failed to load test messages");

    // ACT: Process all messages through the pipeline
    while let Some(message) = source.next_message().await.unwrap() {
        // Process message...
    }

    // ASSERT: Verify expected behavior
    // TODO: Add assertions
}
```

## Test Scenarios

Good test cases include:
- **Timeout resurrection**: Flight times out, then aircraft reappears (should create new flight vs resume)
- **Missed landing**: Aircraft descends, goes out of range while landing (should end flight)
- **Touch-and-go**: Aircraft lands briefly then takes off again (should be one or two flights?)
- **Glider patterns**: Thermal climbing, straight glides, landing patterns
- **Towplane patterns**: Tow climb, release, return to airport

## File Naming Convention

Files are named descriptively based on the test scenario, not the flight ID:
- ✅ `timeout-resurrection-creates-new-flight.txt`
- ✅ `glider-multiple-thermals.txt`
- ❌ `123e4567-e89b-12d3-a456-426614174000-ogn-aprs.txt`

This makes it easy to understand what each test case is testing without needing to look at the code.
