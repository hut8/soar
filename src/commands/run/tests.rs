use chrono::{Datelike, Timelike};

#[test]
fn test_timestamp_parsing_valid() {
    // Test that a valid ISO-8601 timestamp is correctly parsed
    let message = "2025-01-15T12:34:56.789Z FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";

    // We can't directly test process_aprs_message since it's async and requires processors,
    // but we can test the parsing logic
    let (timestamp_str, rest) = message.split_once(' ').unwrap();
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

    assert!(parsed.is_ok());
    let timestamp = parsed.unwrap().with_timezone(&chrono::Utc);
    assert_eq!(timestamp.year(), 2025);
    assert_eq!(timestamp.month(), 1);
    assert_eq!(timestamp.day(), 15);
    assert_eq!(timestamp.hour(), 12);
    assert_eq!(timestamp.minute(), 34);
    assert_eq!(timestamp.second(), 56);
    assert_eq!(
        rest,
        "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322"
    );
}

#[test]
fn test_timestamp_parsing_invalid() {
    // Test that an invalid timestamp doesn't crash
    let message = "INVALID-TIMESTAMP FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E";

    let (timestamp_str, _rest) = message.split_once(' ').unwrap();
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

    assert!(parsed.is_err());
}

#[test]
fn test_timestamp_parsing_missing() {
    // Test that a message without a space (no timestamp) is handled
    let message = "FLRDDA5BA>APRS,qAS,LFNX:/160829h4902.45N/00531.30E'342/049/A=001322";

    let result = message.split_once(' ');
    assert!(result.is_none());
}

#[test]
fn test_timestamp_parsing_server_message() {
    // Test that server messages with timestamps are handled correctly
    let message = "2025-01-15T12:34:56.789Z # aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152";

    let (timestamp_str, rest) = message.split_once(' ').unwrap();
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str);

    assert!(parsed.is_ok());
    assert!(rest.starts_with('#'));
    assert_eq!(
        rest,
        "# aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152"
    );
}

#[test]
fn test_timestamp_format_rfc3339() {
    // Test that Utc::now().to_rfc3339() produces a parseable timestamp
    let now = chrono::Utc::now();
    let timestamp_str = now.to_rfc3339();

    let parsed = chrono::DateTime::parse_from_rfc3339(&timestamp_str);
    assert!(parsed.is_ok());

    let parsed_utc = parsed.unwrap().with_timezone(&chrono::Utc);
    // Should be within 1 second (to account for processing time)
    let diff = (now - parsed_utc).num_milliseconds().abs();
    assert!(diff < 1000);
}
