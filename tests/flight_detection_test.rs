//! Integration test for flight detection using TestMessageSource
//!
//! This test demonstrates how to use the TestMessageSource to replay
//! real APRS message sequences and verify flight detection logic.
//!
//! Test data files are located in tests/data/flights/ and can be generated
//! using the dump-flight-messages.sh script.
use soar::message_sources::{RawMessageSource, TestMessageSource};

/// Example test showing how to read messages from a test file
///
/// This is a basic example showing the TestMessageSource API.
/// Real tests would process these messages through the full flight detection pipeline.
#[tokio::test]
async fn test_message_source_basic_usage() {
    // Create a simple test file
    let test_file_content = r#"2025-01-15T12:00:00.000Z FLRDDA5BA>APRS,qAS,LFNM:/120000h4821.86N/00531.07E'086/007/A=000607
2025-01-15T12:00:05.000Z FLRDDA5BA>APRS,qAS,LFNM:/120005h4821.87N/00531.08E'086/012/A=000650
2025-01-15T12:00:10.000Z FLRDDA5BA>APRS,qAS,LFNM:/120010h4821.88N/00531.09E'086/015/A=000720"#;

    // Write to a temporary file
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file_path = temp_dir.path().join("test-messages.txt");
    std::fs::write(&test_file_path, test_file_content).unwrap();

    // Create a TestMessageSource
    let mut source = TestMessageSource::from_file_with_count(&test_file_path, 3)
        .await
        .unwrap();

    // Verify we can read messages
    assert_eq!(source.remaining_hint(), Some(3));

    let msg1 = source.next_message().await.unwrap();
    assert!(msg1.is_some());
    assert!(msg1.unwrap().contains("FLRDDA5BA"));
    assert_eq!(source.messages_read(), 1);

    let msg2 = source.next_message().await.unwrap();
    assert!(msg2.is_some());
    assert_eq!(source.messages_read(), 2);

    let msg3 = source.next_message().await.unwrap();
    assert!(msg3.is_some());
    assert_eq!(source.messages_read(), 3);

    // End of file
    let msg4 = source.next_message().await.unwrap();
    assert!(msg4.is_none());
}

/// Example showing how to process messages through a simple parser
///
/// This demonstrates the pattern for processing test messages.
/// Real tests would use the full PacketRouter and flight detection pipeline.
#[tokio::test]
async fn test_message_parsing_from_source() {
    let test_file_content = r#"2025-01-15T12:00:00.000Z FLRDDA5BA>APRS,qAS,LFNM:/120000h4821.86N/00531.07E'086/007/A=000607
2025-01-15T12:00:05.000Z FLRDD1234>APRS,qAS,LFNM:/120005h4821.87N/00531.08E'086/012/A=000650"#;

    let temp_dir = tempfile::tempdir().unwrap();
    let test_file_path = temp_dir.path().join("test-parsing.txt");
    std::fs::write(&test_file_path, test_file_content).unwrap();

    let mut source = TestMessageSource::from_file(&test_file_path).await.unwrap();

    let mut messages_processed = 0;

    // Process all messages
    while let Some(message) = source.next_message().await.unwrap() {
        // Extract timestamp and message parts
        let parts: Vec<&str> = message.splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2, "Message should have timestamp and content");

        let timestamp_str = parts[0];
        let aprs_message = parts[1];

        // Verify timestamp format
        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_str);
        assert!(
            timestamp.is_ok(),
            "Timestamp should be valid RFC3339: {}",
            timestamp_str
        );

        // Verify APRS message format
        assert!(
            aprs_message.contains('>') && aprs_message.contains("APRS"),
            "Should be valid APRS message format"
        );

        messages_processed += 1;
    }

    assert_eq!(messages_processed, 2);
}

// NOTE: Full integration tests with database would look like this:
//
// #[tokio::test]
// async fn test_flight_detection_timeout_resurrection() {
//     // Setup test database
//     let pool = setup_test_db().await;
//
//     // Create flight tracker and processors
//     let flight_tracker = FlightTracker::new(&pool);
//     let fix_processor = FixProcessor::new(...);
//
//     // Load test messages from a known problematic flight
//     let mut source = TestMessageSource::from_file(
//         "tests/data/flights/timeout-resurrection/abc123-should-create-new-flight.txt"
//     ).await.unwrap();
//
//     // Process all messages
//     while let Some(message) = source.next_message().await.unwrap() {
//         // Parse and process through the full pipeline
//         process_message(message, &flight_tracker, &fix_processor).await;
//     }
//
//     // Verify the correct number of flights were created
//     let flights = get_flights_for_device(&pool, device_id).await;
//     assert_eq!(flights.len(), 2, "Should create 2 separate flights");
//
//     // Verify flight timestamps and state
//     // ...
// }
