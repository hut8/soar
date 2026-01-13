/// Test for FANET ID parsing
///
/// This test verifies that FANET IDs with the NAVITER format (10 hex digits after "id")
/// are correctly parsed by ogn-parser.
///
/// Issue reference: FANET ID issue with long IDs
/// Example message: FNT1142BB>OGNAVI,qAS,NAVITER2:/114239h4128.47N/08134.48W'000/000/A=000945 !W59! id18501142BB +000fpm +0.0rot
///
/// The ID "id18501142BB" breaks down as:
/// - "id" = prefix (2 chars)
/// - "1850" = detail field (4 hex digits) containing:
///   - Reserved bits: 0
///   - Address type: 5
///   - Aircraft type: 6
///   - No-track flag: false
///   - Stealth flag: false
/// - "1142BB" = actual device address (6 hex digits = 24-bit address)
///
/// The address 0x1142BB = 1,131,195 in decimal, which fits comfortably in a 32-bit signed integer.

#[cfg(test)]
mod fanet_id_tests {
    #[test]
    fn test_fanet_naviter_id_format() {
        // Example FANET message from the issue
        let test_message = "FNT1142BB>OGNAVI,qAS,NAVITER2:/114239h4128.47N/08134.48W'000/000/A=000945 !W59! id18501142BB +000fpm +0.0rot";

        let packet = ogn_parser::parse(test_message).expect("Failed to parse FANET message");

        // Verify basic packet structure
        assert_eq!(packet.from.to_string(), "FNT1142BB");
        assert_eq!(packet.to.to_string(), "OGNAVI");

        // Extract position data
        if let ogn_parser::AprsData::Position(pos) = &packet.data {
            // Verify ID was parsed
            let id = pos.comment.id.as_ref().expect("ID should be present");

            // The address should be 0x1142BB (last 6 hex digits)
            assert_eq!(
                id.address, 0x1142BB,
                "Address should be 0x1142BB (1,131,195 decimal)"
            );

            // Address should fit in i32
            assert!(
                id.address <= i32::MAX as u32,
                "Address must fit in signed 32-bit integer"
            );

            // Verify decoded detail fields from 0x1850
            assert_eq!(id.reserved, Some(0), "Reserved bits should be 0");
            assert_eq!(id.address_type, 5, "Address type should be 5");
            assert_eq!(id.aircraft_type, 6, "Aircraft type should be 6");
            assert_eq!(id.is_notrack, false, "No-track flag should be false");
            assert_eq!(id.is_stealth, false, "Stealth flag should be false");
        } else {
            panic!("Expected Position data in APRS packet");
        }
    }

    #[test]
    fn test_standard_id_format() {
        // Standard 8-character format (idXXYYYYYY) for comparison
        let test_message =
            "FLRDDA5BA>APRS,qAS,LFNM:/074548h4415.61N/00531.90E'342/049/A=001486 !W12! id06DDA5BA -019fpm +0.0rot";

        let packet = ogn_parser::parse(test_message).expect("Failed to parse standard OGN message");

        if let ogn_parser::AprsData::Position(pos) = &packet.data {
            let id = pos.comment.id.as_ref().expect("ID should be present");

            // Standard format has 6-digit address
            assert_eq!(
                id.address, 0xDDA5BA,
                "Address should be 0xDDA5BA (14525882 decimal)"
            );

            // Address type from first byte detail field
            assert_eq!(id.address_type, 2, "Address type should be 2 (Flarm)");

            // Standard format doesn't have reserved field
            assert_eq!(id.reserved, None, "Reserved should be None for standard format");
        } else {
            panic!("Expected Position data in APRS packet");
        }
    }

    #[test]
    fn test_address_fits_in_database() {
        // Verify that typical FANET addresses fit in the database integer column (i32)
        let addresses = vec![
            0x1142BB,   // From FANET example: 1,131,195
            0xDDA5BA,   // From FLARM example: 14,525,882
            0xFFFFFF,   // Max 24-bit address: 16,777,215
            0x500000,   // Mid-range address: 5,242,880
        ];

        for addr in addresses {
            assert!(
                addr <= i32::MAX as u32,
                "Address 0x{:06X} = {} must fit in i32 (max: {})",
                addr,
                addr,
                i32::MAX
            );
        }
    }

    #[test]
    fn test_various_fanet_address_types() {
        // Test that different address types in the detail field are handled correctly
        // Address type is stored in bits 9-4 of the NAVITER format detail field

        // Create test cases with different address types
        let test_cases = vec![
            // (id_string, expected_address_type)
            ("id10001142BB", 0),  // Address type 0
            ("id18501142BB", 5),  // Address type 5 (from issue example)
            ("id1FF01142BB", 63), // Address type 63 (max 6-bit value)
        ];

        for (id_str, expected_addr_type) in test_cases {
            let test_message = format!(
                "FNT123456>OGNAVI,qAS,TEST:/114239h4128.47N/08134.48W'000/000/A=000945 {} +000fpm",
                id_str
            );

            let packet = ogn_parser::parse(&test_message)
                .unwrap_or_else(|_| panic!("Failed to parse message with ID {}", id_str));

            if let ogn_parser::AprsData::Position(pos) = &packet.data {
                let id = pos.comment.id.as_ref().expect("ID should be present");
                assert_eq!(
                    id.address_type, expected_addr_type,
                    "Address type mismatch for ID {}",
                    id_str
                );
            }
        }
    }
}
