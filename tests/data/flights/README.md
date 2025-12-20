# Flight Test Data

This directory contains raw APRS message dumps from real flights, used for integration testing of the flight detection logic.

## File Format

Each file is named `[flight-id]-ogn-aprs.txt` and contains raw APRS messages in chronological order.

**Format:** Each line contains a timestamp and raw APRS message separated by a single space:
```
2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNM:/074548h4821.86N/00531.07E'086/007/A=000607 !W84! id06DDA5BA -019fpm +0.0rot 5.5dB 0e -4.3kHz
```

**Fields:**
- **Timestamp**: ISO 8601 format with UTC timezone (`YYYY-MM-DDTHH:MM:SS.SSSZ`)
- **Raw Message**: Original APRS packet as received from APRS-IS

## Generating Test Data

Use the `dump-flight-messages.sh` script to extract messages from staging or production databases:

```bash
# From staging database
./scripts/dump-flight-messages.sh staging <flight-id>

# From production database (via SSH to glider.flights)
./scripts/dump-flight-messages.sh production <flight-id>
```

## Usage in Tests

These files are used with `TestMessageSource` to replay real message sequences and verify flight detection logic:

```rust
use soar::message_sources::TestMessageSource;

#[tokio::test]
async fn test_flight_detection() {
    let source = TestMessageSource::from_file(
        "tests/data/flights/123e4567-e89b-12d3-a456-426614174000-ogn-aprs.txt"
    ).await.unwrap();

    // Process messages and verify flight detection...
}
```

## Test Case Selection

Good test cases include:
- **Timeout resurrection**: Flight times out, then aircraft reappears (should create new flight vs resume)
- **Missed landing**: Aircraft descends, goes out of range while landing (should end flight)
- **Touch-and-go**: Aircraft lands briefly then takes off again (should be one or two flights?)
- **Glider patterns**: Thermal climbing, straight glides, landing patterns
- **Towplane patterns**: Tow climb, release, return to airport

## File Organization

Consider organizing files by test scenario:
```
flights/
├── README.md
├── timeout-resurrection/
│   ├── abc123-should-create-new-flight.txt
│   └── def456-should-resume-existing.txt
├── landing-detection/
│   ├── ghi789-out-of-range-while-landing.txt
│   └── jkl012-clean-landing.txt
└── complex-patterns/
    └── mno345-multiple-thermals.txt
```
