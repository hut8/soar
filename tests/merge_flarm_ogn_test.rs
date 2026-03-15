//! Integration tests for FLARM/OGN duplicate handling.
//!
//! When the same 24-bit hardware address appears as `flarm_address` on one
//! aircraft row and `ogn_address` on another, the inline merge in
//! `aircraft_for_fix` should skip the doomed UPDATE (which would hit a unique
//! constraint) and let the daily batch process handle it.
mod common;

use common::TestDatabase;
use diesel::prelude::*;
use soar::aircraft::{AddressType, NewAircraft};
use soar::aircraft_repo::{AircraftPacketFields, AircraftRepository};
use soar::schema::aircraft;

/// Helper to create a minimal NewAircraft with only the required fields.
fn new_aircraft() -> NewAircraft {
    NewAircraft {
        icao_address: None,
        flarm_address: None,
        ogn_address: None,
        other_address: None,
        aircraft_model: String::new(),
        registration: None,
        competition_number: String::new(),
        tracked: false,
        identified: false,
        from_ogn_ddb: false,
        from_adsbx_ddb: false,
        frequency_mhz: None,
        pilot_name: None,
        home_base_airport_ident: None,
        last_fix_at: None,
        club_id: None,
        icao_model_code: None,
        adsb_emitter_category: None,
        tracker_device_type: None,
        country_code: None,
        latitude: None,
        longitude: None,
        owner_operator: None,
        aircraft_category: None,
        engine_count: None,
        engine_type: None,
        faa_pia: None,
        faa_ladd: None,
        year: None,
        is_military: None,
        current_fix: None,
        images: None,
        pending_registration: None,
    }
}

fn empty_packet_fields() -> AircraftPacketFields {
    AircraftPacketFields {
        aircraft_category: None,
        aircraft_model: None,
        icao_model_code: None,
        adsb_emitter_category: None,
        tracker_device_type: None,
        registration: None,
    }
}

/// When both a FLARM row and an OGN row exist for the same address,
/// `aircraft_for_fix` should return the OGN row without error and leave
/// both rows intact for the batch merge.
#[tokio::test]
async fn test_aircraft_for_fix_skips_inline_merge_when_duplicate_exists() {
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let pool = test_db.pool();

    let address = 0x4A0501;

    // Insert two rows for the same physical device:
    // one via OGN DDB (flarm_address), one via live packet (ogn_address)
    let mut conn = pool.get().unwrap();
    let flarm_row_id = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            flarm_address: Some(address),
            registration: Some("SP-XSDZ".into()),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert flarm row");

    let ogn_row_id = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            ogn_address: Some(address),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert ogn row");
    drop(conn);

    // Call aircraft_for_fix with OGN address type — this should NOT produce
    // a unique constraint error or a "race condition" warning.
    let repo = AircraftRepository::new(pool.clone());
    let model = repo
        .aircraft_for_fix(address, AddressType::Ogn, empty_packet_fields())
        .await
        .expect("aircraft_for_fix should succeed");

    // Should return the OGN row (matched via ON CONFLICT on ogn_address)
    assert_eq!(model.id, ogn_row_id);
    assert_eq!(model.ogn_address, Some(address));

    // Both rows should still exist — the merge is deferred to the batch process
    let mut conn = pool.get().unwrap();
    let flarm_still_exists = aircraft::table
        .filter(aircraft::id.eq(flarm_row_id))
        .select(aircraft::id)
        .first::<uuid::Uuid>(&mut conn)
        .optional()
        .unwrap();
    assert!(
        flarm_still_exists.is_some(),
        "FLARM row should still exist (deferred to batch merge)"
    );

    let ogn_still_exists = aircraft::table
        .filter(aircraft::id.eq(ogn_row_id))
        .select(aircraft::id)
        .first::<uuid::Uuid>(&mut conn)
        .optional()
        .unwrap();
    assert!(ogn_still_exists.is_some(), "OGN row should still exist");

    // The FLARM row should NOT have gained an ogn_address (no inline merge happened)
    let flarm_ogn: Option<i32> = aircraft::table
        .filter(aircraft::id.eq(flarm_row_id))
        .select(aircraft::ogn_address)
        .first(&mut conn)
        .unwrap();
    assert!(
        flarm_ogn.is_none(),
        "FLARM row should not have ogn_address set (inline merge was skipped)"
    );
}

/// When only one side exists (flarm_address set, no ogn_address row),
/// the inline merge should still work and set ogn_address on the FLARM row.
#[tokio::test]
async fn test_aircraft_for_fix_inline_merge_works_when_no_duplicate() {
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let pool = test_db.pool();

    let address = 0x4A0502;

    // Insert only the FLARM row (from OGN DDB)
    let mut conn = pool.get().unwrap();
    let flarm_row_id = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            flarm_address: Some(address),
            registration: Some("D-KETB".into()),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert flarm row");
    drop(conn);

    // Call aircraft_for_fix with OGN address type — the inline merge should
    // set ogn_address on the existing FLARM row.
    let repo = AircraftRepository::new(pool.clone());
    let model = repo
        .aircraft_for_fix(address, AddressType::Ogn, empty_packet_fields())
        .await
        .expect("aircraft_for_fix should succeed");

    // Should return the FLARM row with ogn_address now set
    assert_eq!(model.id, flarm_row_id);
    assert_eq!(model.flarm_address, Some(address));
    assert_eq!(model.ogn_address, Some(address));
}
