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

/// Test case: Aircraft descended out of range while landing, then took off hours later
///
/// **Scenario:**
/// This test covers a canonical case of flight coalescing detection failure. The aircraft:
/// 1. Started descending from FL182 at -1664fpm
/// 2. Continued descending to FL034.56 at -896fpm
/// 3. Went out of range at 17:28:16 UTC (last fix while descending)
/// 4. Gap of 11.3 hours with only 32km horizontal movement
/// 5. Reappeared at 04:46:34 UTC climbing at +3392fpm at FL046.51
///
/// **Current behavior (WRONG):**
/// - Creates ONE excessively long flight spanning 11+ hours
///
/// **Expected behavior (CORRECT):**
/// - Creates TWO separate flights:
///   - Flight 1: Ends when aircraft descended out of range (landed)
///   - Flight 2: Starts when aircraft reappeared climbing (new takeoff)
///
/// **Detection criteria:**
/// - Average descent rate over last 10 fixes before gap was significant (-896fpm+)
/// - Long gap (11+ hours) with minimal horizontal movement (32km)
/// - Reappeared in climbing state (+3392fpm) only a short distance away
/// - Clear indication of landing → ground time → new takeoff
///
/// Flight ID: 019b4d4a-a428-76f0-8e15-1fe429f4254c
/// Environment: production
/// Messages: 1183
/// Device: ICA48683E
/// Gap: 2025-12-23T17:28:16Z to 2025-12-24T04:46:34Z (11h 18m)
/// Distance during gap: 32.29 km
/// Generated: 2025-12-24 08:00:53 UTC
#[tokio::test]
async fn test_descended_out_of_range_while_landing_then_took_off_hours_later() {
    use diesel::PgConnection;
    use diesel::r2d2::{ConnectionManager, Pool};
    use soar::fix_processor::FixProcessor;
    use soar::message_sources::{RawMessageSource, TestMessageSource};
    use soar::packet_processors::generic::GenericProcessor;
    use soar::raw_messages_repo::RawMessagesRepository;
    use soar::receiver_repo::ReceiverRepository;

    // ========== ARRANGE ==========

    // Set up test database
    let database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set for tests");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create pool");

    // Create repositories and processors
    let receiver_repo = ReceiverRepository::new(pool.clone());
    let raw_messages_repo = RawMessagesRepository::new(pool.clone());
    let generic_processor = GenericProcessor::new(receiver_repo, raw_messages_repo);

    // Set up elevation service for AGL calculation (required for flight creation)
    let elevation_service = soar::elevation::ElevationService::new_with_s3()
        .await
        .expect("Failed to create elevation service");

    // Enable tracing for debugging
    use tracing_subscriber::EnvFilter;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("soar::flight_tracker=debug".parse().unwrap()),
        )
        .try_init();

    let fix_processor = FixProcessor::new(pool.clone()).with_sync_elevation(elevation_service);

    // Load test messages
    let mut source = TestMessageSource::from_file(
        "tests/data/flights/descended-out-of-range-while-landing-then-took-off-hours-later.txt",
    )
    .await
    .expect("Failed to load test messages");

    println!("📝 Starting to process 1183 messages...");

    // ========== ACT ==========

    let mut messages_processed = 0;
    let mut first_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;

    while let Some(message) = source.next_message().await.unwrap() {
        // Parse message: "YYYY-MM-DDTHH:MM:SS.SSSZ <aprs_message>"
        let (timestamp_str, aprs_message) = message
            .split_once(' ')
            .expect("Message should have timestamp");

        let received_at = chrono::DateTime::parse_from_rfc3339(timestamp_str)
            .expect("Valid RFC3339 timestamp")
            .with_timezone(&chrono::Utc);

        // Track timestamp range for later queries
        if first_timestamp.is_none() {
            first_timestamp = Some(received_at);
        }
        last_timestamp = Some(received_at);

        // Parse APRS packet
        if let Ok(packet) = ogn_parser::parse(aprs_message) {
            // First, process through generic processor to ensure aircraft/receiver exist and get context
            if let Some(context) = generic_processor
                .process_packet(&packet, aprs_message, received_at)
                .await
            {
                // Process through fix processor with the context from generic processor
                fix_processor
                    .process_aprs_packet(packet, aprs_message, context)
                    .await;
            }
        }

        messages_processed += 1;
        if messages_processed % 100 == 0 {
            println!("   Processed {} messages...", messages_processed);
        }
    }

    println!("✅ Processed all {} messages", messages_processed);

    // ========== ASSERT ==========

    let first_ts = first_timestamp.expect("Should have processed at least one message");
    let last_ts = last_timestamp.expect("Should have processed at least one message");

    // Debug: Check if fixes were created
    use diesel::dsl::count_star;
    use diesel::prelude::*;
    use soar::schema::{aircraft, fixes};

    let mut conn = pool.get().expect("Failed to get database connection");

    let fix_count: i64 = fixes::table
        .select(count_star())
        .first(&mut conn)
        .expect("Failed to count fixes");
    println!("📊 Found {} fixes in database", fix_count);

    let aircraft_count: i64 = aircraft::table
        .select(count_star())
        .first(&mut conn)
        .expect("Failed to count aircraft");
    println!("📊 Found {} aircraft in database", aircraft_count);

    // Check total flights in database (not just time range)
    use soar::schema::flights;
    let total_flights: i64 = flights::table
        .select(count_star())
        .first(&mut conn)
        .expect("Failed to count flights");
    println!(
        "📊 Found {} total flights in database (all time)",
        total_flights
    );

    // Query flights in the time range of our test messages
    let flights_repo = soar::flights_repo::FlightsRepository::new(pool.clone());
    let flights = flights_repo
        .get_flights_in_time_range(first_ts, last_ts, None)
        .await
        .expect("Failed to query flights");

    println!(
        "\n📊 Found {} flight(s) in time range {} to {}",
        flights.len(),
        first_ts.format("%Y-%m-%d %H:%M:%S"),
        last_ts.format("%Y-%m-%d %H:%M:%S")
    );

    for (i, flight) in flights.iter().enumerate() {
        let takeoff = flight
            .takeoff_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "In-flight start".to_string());
        let landing = flight
            .landing_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .or_else(|| {
                flight
                    .timed_out_at
                    .map(|t| format!("{} (timeout)", t.format("%Y-%m-%d %H:%M:%S")))
            })
            .unwrap_or_else(|| "In progress".to_string());
        println!("   Flight {}: {} -> {}", i + 1, takeoff, landing);
    }

    // CRITICAL ASSERTION: Should create TWO flights, not one
    assert_eq!(
        flights.len(),
        2,
        "Should create 2 separate flights (one ending at landing, one starting at takeoff), not 1 long flight"
    );

    // Verify first flight ended (either landed or timed out)
    let flight1 = &flights[0];
    assert!(
        flight1.landing_time.is_some() || flight1.timed_out_at.is_some(),
        "First flight should have ended when aircraft descended out of range"
    );

    // Verify second flight started later
    let flight2 = &flights[1];
    let flight1_end = flight1
        .landing_time
        .or(flight1.timed_out_at)
        .expect("Flight 1 should have ended");
    let flight2_start = flight2.takeoff_time.unwrap_or(flight2.last_fix_at);
    let gap = (flight2_start - flight1_end).num_seconds();
    assert!(
        gap > 10 * 3600,
        "Gap between flights should be over 10 hours (actual: {} seconds = {:.1} hours)",
        gap,
        gap as f64 / 3600.0
    );

    println!("\n✅ Test passed: Two separate flights detected correctly");

    let flight1_start_str = flight1
        .takeoff_time
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| {
            flight1
                .last_fix_at
                .format("%H:%M:%S (in-flight)")
                .to_string()
        });
    let flight1_end_str = flight1_end.format("%H:%M:%S").to_string();
    let flight2_start_str = flight2_start.format("%H:%M:%S").to_string();

    println!("   Flight 1: {} -> {}", flight1_start_str, flight1_end_str);
    println!("   Gap: {:.1} hours", gap as f64 / 3600.0);
    println!("   Flight 2: {} -> ...", flight2_start_str);
}
