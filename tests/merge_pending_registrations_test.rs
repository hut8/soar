//! Integration tests for merge_pending_registrations()
//!
//! These tests verify that duplicate aircraft with pending registrations are
//! correctly merged into their target aircraft, including edge cases around
//! the chk_at_least_one_address constraint.
mod common;

use common::TestDatabase;
use diesel::prelude::*;
use soar::aircraft::NewAircraft;
use soar::aircraft_repo::AircraftRepository;
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

/// Regression test: a duplicate whose only address is icao_address must be
/// merged without violating chk_at_least_one_address.
///
/// Previously, the code NULLed out icao_address on the duplicate before
/// deleting it, which violated the constraint when icao_address was the
/// duplicate's sole address.
#[tokio::test]
async fn test_merge_duplicate_with_only_icao_address() {
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let pool = test_db.pool();

    // Insert the target aircraft (owns the registration, has a flarm address)
    let target = {
        let mut conn = pool.get().unwrap();
        diesel::insert_into(aircraft::table)
            .values(NewAircraft {
                flarm_address: Some(0xDD1234),
                registration: Some("N204DG".into()),
                ..new_aircraft()
            })
            .returning(aircraft::id)
            .get_result::<uuid::Uuid>(&mut conn)
            .expect("insert target")
    };

    // Insert the duplicate (only has icao_address, pending merge into target)
    let dup = {
        let mut conn = pool.get().unwrap();
        diesel::insert_into(aircraft::table)
            .values(NewAircraft {
                icao_address: Some(0xABCDEF),
                pending_registration: Some("N204DG".into()),
                ..new_aircraft()
            })
            .returning(aircraft::id)
            .get_result::<uuid::Uuid>(&mut conn)
            .expect("insert duplicate")
    };

    // Run the merge
    let repo = AircraftRepository::new(pool.clone());
    let stats = repo
        .merge_pending_registrations()
        .await
        .expect("merge_pending_registrations should succeed");

    assert_eq!(stats.duplicates_found, 1);
    assert_eq!(stats.aircraft_merged, 1);
    assert_eq!(stats.aircraft_deleted, 1);
    assert!(stats.errors.is_empty(), "errors: {:?}", stats.errors);

    // Verify the duplicate is gone
    let mut conn = pool.get().unwrap();
    let dup_exists = aircraft::table
        .filter(aircraft::id.eq(dup))
        .select(aircraft::id)
        .first::<uuid::Uuid>(&mut conn)
        .optional()
        .unwrap();
    assert!(dup_exists.is_none(), "duplicate should be deleted");

    // Verify the target now has the icao_address from the duplicate
    let (target_icao, target_flarm): (Option<i32>, Option<i32>) = aircraft::table
        .filter(aircraft::id.eq(target))
        .select((aircraft::icao_address, aircraft::flarm_address))
        .first(&mut conn)
        .unwrap();
    assert_eq!(target_icao, Some(0xABCDEF));
    assert_eq!(target_flarm, Some(0xDD1234));
}

/// When the target already has an icao_address, the duplicate's icao_address
/// should be discarded (not transferred) and the merge should still succeed.
#[tokio::test]
async fn test_merge_duplicate_target_already_has_address() {
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let pool = test_db.pool();

    let mut conn = pool.get().unwrap();

    // Target already has icao_address
    let target = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            icao_address: Some(0x111111),
            registration: Some("N501DG".into()),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert target");

    // Duplicate also has icao_address (different value) — it should be discarded
    let _dup = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            icao_address: Some(0x222222),
            pending_registration: Some("N501DG".into()),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert duplicate");

    drop(conn);

    let repo = AircraftRepository::new(pool.clone());
    let stats = repo
        .merge_pending_registrations()
        .await
        .expect("merge should succeed");

    assert_eq!(stats.aircraft_merged, 1);
    assert!(stats.errors.is_empty(), "errors: {:?}", stats.errors);

    // Target should retain its original icao_address
    let mut conn = pool.get().unwrap();
    let target_icao: Option<i32> = aircraft::table
        .filter(aircraft::id.eq(target))
        .select(aircraft::icao_address)
        .first(&mut conn)
        .unwrap();
    assert_eq!(target_icao, Some(0x111111));
}

/// Multiple address types on the duplicate should all transfer to the target
/// if the target lacks them.
#[tokio::test]
async fn test_merge_transfers_multiple_address_types() {
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let pool = test_db.pool();

    let mut conn = pool.get().unwrap();

    // Target only has flarm_address
    let target = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            flarm_address: Some(0xAAAAAA),
            registration: Some("N503DG".into()),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert target");

    // Duplicate has icao + ogn addresses
    let _dup = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            icao_address: Some(0xBBBBBB),
            ogn_address: Some(0xCCCCCC),
            pending_registration: Some("N503DG".into()),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert duplicate");

    drop(conn);

    let repo = AircraftRepository::new(pool.clone());
    let stats = repo
        .merge_pending_registrations()
        .await
        .expect("merge should succeed");

    assert_eq!(stats.aircraft_merged, 1);
    assert!(stats.errors.is_empty(), "errors: {:?}", stats.errors);

    let mut conn = pool.get().unwrap();
    let (icao, flarm, ogn): (Option<i32>, Option<i32>, Option<i32>) = aircraft::table
        .filter(aircraft::id.eq(target))
        .select((
            aircraft::icao_address,
            aircraft::flarm_address,
            aircraft::ogn_address,
        ))
        .first(&mut conn)
        .unwrap();
    assert_eq!(icao, Some(0xBBBBBB));
    assert_eq!(flarm, Some(0xAAAAAA));
    assert_eq!(ogn, Some(0xCCCCCC));
}

/// When no target aircraft owns the pending registration, the duplicate should
/// claim the registration directly (no merge, no deletion).
#[tokio::test]
async fn test_merge_no_target_claims_registration() {
    let test_db = TestDatabase::new()
        .await
        .expect("Failed to create test database");
    let pool = test_db.pool();

    let mut conn = pool.get().unwrap();

    // Insert aircraft with pending_registration but no target owns "N999ZZ"
    let aircraft_id = diesel::insert_into(aircraft::table)
        .values(NewAircraft {
            icao_address: Some(0x333333),
            pending_registration: Some("N999ZZ".into()),
            ..new_aircraft()
        })
        .returning(aircraft::id)
        .get_result::<uuid::Uuid>(&mut conn)
        .expect("insert aircraft");

    drop(conn);

    let repo = AircraftRepository::new(pool.clone());
    let stats = repo
        .merge_pending_registrations()
        .await
        .expect("merge should succeed");

    assert_eq!(stats.duplicates_found, 1);
    assert_eq!(stats.registrations_claimed, 1);
    assert_eq!(stats.aircraft_merged, 0);
    assert_eq!(stats.aircraft_deleted, 0);
    assert!(stats.errors.is_empty(), "errors: {:?}", stats.errors);

    // Aircraft should now have the registration set and pending cleared
    let mut conn = pool.get().unwrap();
    let (reg, pending): (Option<String>, Option<String>) = aircraft::table
        .filter(aircraft::id.eq(aircraft_id))
        .select((aircraft::registration, aircraft::pending_registration))
        .first(&mut conn)
        .unwrap();
    assert_eq!(reg.as_deref(), Some("N999ZZ"));
    assert!(pending.is_none());
}
