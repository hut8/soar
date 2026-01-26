use anyhow::{Context, Result};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use fake::Fake;
use fake::faker::company::en::*;
use fake::faker::internet::en::*;
use fake::faker::name::en::*;
use tracing::info;
use uuid::Uuid;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Seed test data for E2E testing
///
/// This command creates a known set of test data that E2E tests can rely on.
/// It uses the `fake` crate to generate realistic but deterministic test data.
///
/// Environment variables:
/// - TEST_USER_EMAIL: Email for test user (default: test@example.com)
/// - TEST_USER_PASSWORD: Password for test user (default: testpassword123)
/// - SEED_COUNT: Number of additional fake records to create (default: 10)
pub async fn handle_seed_test_data(pool: &PgPool) -> Result<()> {
    info!("Starting test data seed");

    let mut conn = pool.get().context("Failed to get database connection")?;

    // Get configuration from environment variables
    let test_email =
        std::env::var("TEST_USER_EMAIL").unwrap_or_else(|_| "test@example.com".to_string());
    let test_password =
        std::env::var("TEST_USER_PASSWORD").unwrap_or_else(|_| "testpassword123".to_string());
    let seed_count: usize = std::env::var("SEED_COUNT")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);

    // Hash the test password using Argon2 (matching the authentication system)
    let password_hash = hash_password(&test_password)?;

    // Create test clubs
    info!("Creating test clubs");
    let test_club_id = create_test_clubs(&mut conn, seed_count)?;

    // Create test user
    info!("Creating test user: {}", test_email);
    let _user_id = create_test_user(
        &mut conn,
        &test_email,
        &password_hash,
        "Test",
        "User",
        test_club_id,
    )?;

    // Create additional fake users
    info!("Creating {} fake users", seed_count);
    create_fake_users(&mut conn, test_club_id, seed_count)?;

    // Create test pilots
    info!("Creating test pilots");
    create_test_pilots(&mut conn, test_club_id, seed_count)?;

    // Create test aircraft
    info!("Creating test aircraft");
    let device_ids = create_test_devices(&mut conn, seed_count)?;

    // Create test flights and fixes for the first test device
    if !device_ids.is_empty() {
        info!("Creating test flights and position fixes");
        create_test_flights_and_fixes(&mut conn, device_ids[0])?;
    }

    info!("Test data seed completed successfully");
    info!("Test user credentials:");
    info!("  Email: {}", test_email);
    info!("  Password: {}", test_password);
    info!("  Club ID: {}", test_club_id);

    Ok(())
}

/// Hash a password using Argon2 (matching the production authentication system)
fn hash_password(password: &str) -> Result<String> {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    Ok(password_hash.to_string())
}

fn create_test_clubs(conn: &mut PgConnection, count: usize) -> Result<Uuid> {
    use soar::schema::clubs::dsl::*;

    // Create the primary test club with a deterministic UUID
    // This ensures E2E tests can rely on a known club ID
    let test_club_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")
        .expect("Failed to parse test club UUID");
    let test_club_name = "Test Soaring Club";

    diesel::insert_into(clubs)
        .values((
            id.eq(test_club_id),
            name.eq(test_club_name),
            is_soaring.eq(Some(true)),
            created_at.eq(chrono::Utc::now()),
            updated_at.eq(chrono::Utc::now()),
        ))
        .on_conflict(id)
        .do_update()
        .set((
            name.eq(test_club_name),
            is_soaring.eq(Some(true)),
            updated_at.eq(chrono::Utc::now()),
        ))
        .execute(conn)
        .context("Failed to create test club")?;

    info!(
        "Created primary test club: {} (ID: {})",
        test_club_name, test_club_id
    );

    // Create additional fake clubs
    for _ in 0..count {
        let club_name: String = CompanyName().fake();
        let club_id = Uuid::new_v4();

        diesel::insert_into(clubs)
            .values((
                id.eq(club_id),
                name.eq(format!("{} Soaring Club", club_name)),
                is_soaring.eq(Some(true)),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .ok();
    }

    Ok(test_club_id)
}

fn create_test_user(
    conn: &mut PgConnection,
    email_address: &str,
    password_hash_value: &str,
    first_name_value: &str,
    last_name_value: &str,
    club_id_value: Uuid,
) -> Result<Uuid> {
    use soar::schema::users::dsl::*;

    let user_id = Uuid::new_v4();

    diesel::insert_into(users)
        .values((
            id.eq(user_id),
            first_name.eq(first_name_value),
            last_name.eq(last_name_value),
            email.eq(email_address),
            password_hash.eq(password_hash_value),
            is_admin.eq(false),
            club_id.eq(Some(club_id_value)),
            email_verified.eq(true),
            is_licensed.eq(false),
            is_instructor.eq(false),
            is_tow_pilot.eq(false),
            is_examiner.eq(false),
            settings.eq(serde_json::json!({})),
            created_at.eq(chrono::Utc::now()),
            updated_at.eq(chrono::Utc::now()),
        ))
        .on_conflict(id)
        .do_update()
        .set((
            first_name.eq(first_name_value),
            last_name.eq(last_name_value),
            email.eq(email_address),
            password_hash.eq(password_hash_value),
            club_id.eq(Some(club_id_value)),
            email_verified.eq(true),
            updated_at.eq(chrono::Utc::now()),
        ))
        .execute(conn)
        .context("Failed to create test user")?;

    info!("Created test user: {} (ID: {})", email_address, user_id);
    Ok(user_id)
}

fn create_fake_users(conn: &mut PgConnection, club_id_value: Uuid, count: usize) -> Result<()> {
    use soar::schema::users::dsl::*;

    for _ in 0..count {
        let user_id = Uuid::new_v4();
        let user_first_name: String = FirstName().fake();
        let user_last_name: String = LastName().fake();
        let user_email: String = FreeEmail().fake();
        let default_password_hash = hash_password("password123")?;

        diesel::insert_into(users)
            .values((
                id.eq(user_id),
                first_name.eq(user_first_name),
                last_name.eq(user_last_name),
                email.eq(user_email),
                password_hash.eq(default_password_hash),
                is_admin.eq(false),
                club_id.eq(Some(club_id_value)),
                email_verified.eq(true),
                is_licensed.eq(false),
                is_instructor.eq(false),
                is_tow_pilot.eq(false),
                is_examiner.eq(false),
                settings.eq(serde_json::json!({})),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .ok();
    }

    Ok(())
}

fn create_test_pilots(conn: &mut PgConnection, club_id_value: Uuid, count: usize) -> Result<()> {
    use soar::schema::users::dsl::*;

    // Create a mix of licensed/unlicensed, instructor/student pilots (no email/password - pilot-only users)
    for i in 0..count {
        let pilot_id = Uuid::new_v4();
        let pilot_first_name: String = FirstName().fake();
        let pilot_last_name: String = LastName().fake();
        let is_licensed_pilot = i % 3 != 0; // 2/3 are licensed
        let is_instructor_pilot = i % 5 == 0; // 1/5 are instructors
        let is_tow_pilot_flag = i % 7 == 0; // 1/7 are tow pilots

        diesel::insert_into(users)
            .values((
                id.eq(pilot_id),
                first_name.eq(pilot_first_name),
                last_name.eq(pilot_last_name),
                email.eq(None::<String>), // No email - pilot-only user
                password_hash.eq(None::<String>), // No password
                is_admin.eq(false),
                is_licensed.eq(is_licensed_pilot),
                is_instructor.eq(is_instructor_pilot),
                is_tow_pilot.eq(is_tow_pilot_flag),
                is_examiner.eq(false),
                club_id.eq(Some(club_id_value)),
                email_verified.eq(false),
                settings.eq(serde_json::json!({})),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .ok();
    }

    Ok(())
}

fn create_test_devices(conn: &mut PgConnection, count: usize) -> Result<Vec<Uuid>> {
    use soar::aircraft::AddressType;
    use soar::schema::aircraft::dsl::*;

    let mut device_ids = Vec::new();

    // Define known test aircraft
    let known_devices = vec![
        ("N12345", "ABC123", "ASK-21"),
        ("N54321", "DEF456", "Discus-2c"),
        ("N98765", "GHI789", "ASG-29"),
    ];

    for (reg, addr, model) in known_devices {
        let aircraft_id = Uuid::new_v4();
        let addr_int: i32 = i32::from_str_radix(addr, 16).unwrap_or(0);

        let result = diesel::insert_into(aircraft)
            .values((
                id.eq(aircraft_id),
                address.eq(addr_int),
                address_type.eq(AddressType::Icao),
                registration.eq(reg),
                aircraft_model.eq(model),
                competition_number.eq(""),
                tracked.eq(true),
                identified.eq(true),
                from_ogn_ddb.eq(false),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict((address_type, address))
            .do_update()
            .set((
                registration.eq(reg),
                aircraft_model.eq(model),
                updated_at.eq(chrono::Utc::now()),
            ))
            .execute(conn);

        match result {
            Ok(_) => {
                info!("Created/updated test device: {} ({})", reg, addr);
                // Query back the actual device ID after upsert
                let actual_device_id: Uuid = aircraft
                    .filter(address_type.eq(AddressType::Icao))
                    .filter(address.eq(addr_int))
                    .select(id)
                    .first(conn)
                    .expect("Failed to query device ID after upsert");
                device_ids.push(actual_device_id);
            }
            Err(e) => tracing::error!("Failed to create test device {} ({}): {}", reg, addr, e),
        }
    }

    // Create additional random aircraft
    for i in 0..count {
        let aircraft_id = Uuid::new_v4();
        let reg_number = format!("N{:05}", 10000 + i);
        let hex_addr = format!("{:06X}", 100000 + i);
        let addr_int: i32 = i32::from_str_radix(&hex_addr, 16).unwrap_or(0);
        let models = [
            "ASK-21",
            "Discus-2c",
            "ASG-29",
            "Duo Discus",
            "Arcus",
            "Diana 2",
        ];
        let model = models[i % models.len()];

        diesel::insert_into(aircraft)
            .values((
                id.eq(aircraft_id),
                address.eq(addr_int),
                address_type.eq(AddressType::Icao),
                registration.eq(reg_number.clone()),
                aircraft_model.eq(model),
                competition_number.eq(""),
                tracked.eq(i % 2 == 0), // Half are tracked
                identified.eq(true),
                from_ogn_ddb.eq(false),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict((address_type, address))
            .do_update()
            .set((
                registration.eq(reg_number),
                aircraft_model.eq(model),
                updated_at.eq(chrono::Utc::now()),
            ))
            .execute(conn)
            .ok();
    }

    Ok(device_ids)
}

fn create_test_flights_and_fixes(conn: &mut PgConnection, test_device_id: Uuid) -> Result<()> {
    use chrono::{Duration, Utc};
    use soar::aircraft::AddressType;
    use soar::schema::fixes;
    use soar::schema::flights;

    // Create a recent flight (within last 2 days)
    let new_flight_id = Uuid::new_v4();
    let flight_start = Utc::now() - Duration::days(1) - Duration::hours(2);
    let flight_end = Utc::now() - Duration::days(1);

    diesel::insert_into(flights::table)
        .values((
            flights::id.eq(new_flight_id),
            flights::aircraft_id.eq(Some(test_device_id)),
            flights::device_address.eq("ABC123"),
            flights::device_address_type.eq(AddressType::Icao),
            flights::takeoff_time.eq(Some(flight_start)),
            flights::landing_time.eq(Some(flight_end)),
            flights::last_fix_at.eq(flight_end),
            flights::created_at.eq(Utc::now()),
            flights::updated_at.eq(Utc::now()),
        ))
        .on_conflict_do_nothing()
        .execute(conn)
        .ok();

    // Create position fixes for the flight
    // Based on production data: source is like "ICA342348", aprs_type is "OGADSB", via is like {qAS,AVX920}
    for i in 0..10 {
        let fix_time = flight_start + Duration::minutes(i * 6);
        // Create a flight path around San Francisco area
        let lat = 37.5 + (i as f64 * 0.001);
        let lon = -122.0 + (i as f64 * 0.001);

        diesel::insert_into(fixes::table)
            .values((
                fixes::aircraft_id.eq(test_device_id),
                fixes::flight_id.eq(Some(new_flight_id)),
                fixes::latitude.eq(lat),
                fixes::longitude.eq(lon),
                fixes::altitude_msl_feet.eq(Some(2000 + (i as i32 * 100))),
                fixes::ground_speed_knots.eq(Some(45.0 + (i as f32 * 2.0))),
                fixes::track_degrees.eq(Some(90.0 + (i as f32 * 5.0))),
                // Use realistic values based on production data
                fixes::source.eq("ICAABC123"), // Source is the device address with ICAO prefix
                fixes::source_metadata.eq(serde_json::json!({
                    "protocol": "aprs",
                    "aprs_type": "OGADSB",
                    "via": ["qAS", "TestStation"]
                })),
                fixes::received_at.eq(fix_time),
                fixes::is_active.eq(true),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .ok();
    }

    info!("Created test flight and {} position fixes for device", 10);
    Ok(())
}
