use anyhow::{Context, Result};
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

    // Hash the test password
    let password_hash =
        bcrypt::hash(&test_password, bcrypt::DEFAULT_COST).context("Failed to hash password")?;

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

    // Create test devices
    info!("Creating test devices");
    create_test_devices(&mut conn, seed_count)?;

    info!("Test data seed completed successfully");
    info!("Test user credentials:");
    info!("  Email: {}", test_email);
    info!("  Password: {}", test_password);
    info!("  Club ID: {}", test_club_id);

    Ok(())
}

fn create_test_clubs(conn: &mut PgConnection, count: usize) -> Result<Uuid> {
    use soar::schema::clubs::dsl::*;

    // Create the primary test club
    let test_club_id = Uuid::new_v4();
    let test_club_name = "Test Soaring Club";

    diesel::insert_into(clubs)
        .values((
            id.eq(test_club_id),
            name.eq(test_club_name),
            is_soaring.eq(Some(true)),
            created_at.eq(chrono::Utc::now()),
            updated_at.eq(chrono::Utc::now()),
        ))
        .on_conflict(name)
        .do_update()
        .set((is_soaring.eq(Some(true)), updated_at.eq(chrono::Utc::now())))
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
            settings.eq(serde_json::json!({})),
            created_at.eq(chrono::Utc::now()),
            updated_at.eq(chrono::Utc::now()),
        ))
        .on_conflict(email)
        .do_update()
        .set((
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
        let default_password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST)?;

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
    use soar::schema::pilots::dsl::*;

    // Create a mix of licensed/unlicensed, instructor/student pilots
    for i in 0..count {
        let pilot_id = Uuid::new_v4();
        let pilot_first_name: String = FirstName().fake();
        let pilot_last_name: String = LastName().fake();
        let is_licensed_pilot = i % 3 != 0; // 2/3 are licensed
        let is_instructor_pilot = i % 5 == 0; // 1/5 are instructors
        let is_tow_pilot_flag = i % 7 == 0; // 1/7 are tow pilots

        diesel::insert_into(pilots)
            .values((
                id.eq(pilot_id),
                first_name.eq(pilot_first_name),
                last_name.eq(pilot_last_name),
                is_licensed.eq(is_licensed_pilot),
                is_instructor.eq(is_instructor_pilot),
                is_tow_pilot.eq(is_tow_pilot_flag),
                is_examiner.eq(false),
                club_id.eq(Some(club_id_value)),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .ok();
    }

    Ok(())
}

fn create_test_devices(conn: &mut PgConnection, count: usize) -> Result<()> {
    use soar::devices::AddressType;
    use soar::schema::devices::dsl::*;

    // Define known test devices
    let known_devices = vec![
        ("N12345", "ABC123", "ASK-21"),
        ("N54321", "DEF456", "Discus-2c"),
        ("N98765", "GHI789", "ASG-29"),
    ];

    for (reg, addr, model) in known_devices {
        let device_id = Uuid::new_v4();
        let addr_int: i32 = i32::from_str_radix(addr, 16).unwrap_or(0);

        diesel::insert_into(devices)
            .values((
                id.eq(device_id),
                address.eq(addr_int),
                address_type.eq(AddressType::Icao),
                registration.eq(reg),
                aircraft_model.eq(model),
                competition_number.eq(""),
                tracked.eq(true),
                identified.eq(true),
                from_ddb.eq(false),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict(address)
            .do_nothing()
            .execute(conn)
            .ok();

        info!("Created test device: {} ({})", reg, addr);
    }

    // Create additional random devices
    for i in 0..count {
        let device_id = Uuid::new_v4();
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

        diesel::insert_into(devices)
            .values((
                id.eq(device_id),
                address.eq(addr_int),
                address_type.eq(AddressType::Icao),
                registration.eq(reg_number),
                aircraft_model.eq(model),
                competition_number.eq(""),
                tracked.eq(i % 2 == 0), // Half are tracked
                identified.eq(true),
                from_ddb.eq(false),
                created_at.eq(chrono::Utc::now()),
                updated_at.eq(chrono::Utc::now()),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .ok();
    }

    Ok(())
}
