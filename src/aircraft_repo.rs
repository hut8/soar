use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use tracing::{info, warn};
use uuid::Uuid;

use crate::aircraft::{AddressType, Aircraft, AircraftModel, NewAircraft};
use crate::ogn_aprs_aircraft::{AdsbEmitterCategory, AircraftType};
use crate::schema::aircraft;
use chrono::{DateTime, Utc};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Fields extracted from packet for device creation/update
#[derive(Debug, Clone)]
pub struct AircraftPacketFields {
    pub aircraft_type: Option<AircraftType>,
    pub icao_model_code: Option<String>,
    pub adsb_emitter_category: Option<AdsbEmitterCategory>,
    pub tracker_device_type: Option<String>,
    pub registration: Option<String>,
}

#[derive(Clone)]
pub struct AircraftRepository {
    pool: PgPool,
}

impl AircraftRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn get_connection(&self) -> Result<PgPooledConnection> {
        self.pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))
    }

    /// Upsert aircraft into the database
    /// This will insert new aircraft or update existing ones based on aircraft_id
    pub async fn upsert_aircraft<I>(&self, aircraft_iter: I) -> Result<usize>
    where
        I: IntoIterator<Item = Aircraft>,
    {
        let mut conn = self.get_connection()?;
        let mut upserted_count = 0;

        // Convert aircraft to NewAircraft structs for insertion
        let new_aircraft: Vec<NewAircraft> = aircraft_iter.into_iter().map(|d| d.into()).collect();

        for new_aircraft_entry in new_aircraft {
            let result = diesel::insert_into(aircraft::table)
                .values(&new_aircraft_entry)
                .on_conflict((aircraft::address_type, aircraft::address))
                .do_update()
                .set((
                    // Update fields from DDB, but preserve existing values if DDB value is empty
                    // Use COALESCE(NULLIF(new, ''), old) to keep existing data when DDB has empty strings
                    aircraft::address_type.eq(excluded(aircraft::address_type)),
                    aircraft::aircraft_model.eq(diesel::dsl::sql(
                        "COALESCE(NULLIF(EXCLUDED.aircraft_model, ''), aircraft.aircraft_model)"
                    )),
                    aircraft::registration.eq(diesel::dsl::sql(
                        "COALESCE(NULLIF(EXCLUDED.registration, ''), aircraft.registration)"
                    )),
                    aircraft::competition_number.eq(diesel::dsl::sql(
                        "COALESCE(NULLIF(EXCLUDED.competition_number, ''), aircraft.competition_number)"
                    )),
                    aircraft::tracked.eq(excluded(aircraft::tracked)),
                    aircraft::identified.eq(excluded(aircraft::identified)),
                    aircraft::from_ddb.eq(excluded(aircraft::from_ddb)),
                    // For Option fields, use COALESCE to prefer new value over NULL, but keep old if new is NULL
                    aircraft::frequency_mhz.eq(diesel::dsl::sql(
                        "COALESCE(EXCLUDED.frequency_mhz, aircraft.frequency_mhz)"
                    )),
                    aircraft::pilot_name.eq(diesel::dsl::sql(
                        "COALESCE(EXCLUDED.pilot_name, aircraft.pilot_name)"
                    )),
                    aircraft::home_base_airport_ident.eq(diesel::dsl::sql(
                        "COALESCE(EXCLUDED.home_base_airport_ident, aircraft.home_base_airport_ident)"
                    )),
                    aircraft::updated_at.eq(diesel::dsl::now),
                    // NOTE: We do NOT update the following fields because they come from real-time packets:
                    // - aircraft_type_ogn (from OGN packets)
                    // - icao_model_code (from ADSB packets)
                    // - adsb_emitter_category (from ADSB packets)
                    // - tracker_device_type (from tracker packets)
                    // - country_code (derived from ICAO address, managed separately)
                    // - last_fix_at (managed by fix processing)
                    // - club_id (managed by club assignment logic)
                ))
                .execute(&mut conn);

            match result {
                Ok(_) => {
                    upserted_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to upsert aircraft {}: {}",
                        new_aircraft_entry.address, e
                    );
                    // Continue with other aircraft rather than failing the entire batch
                }
            }
        }

        info!("Successfully upserted {} aircraft", upserted_count);
        Ok(upserted_count)
    }

    /// Get the total count of aircraft in the database
    pub async fn get_aircraft_count(&self) -> Result<i64> {
        let mut conn = self.get_connection()?;
        let count = aircraft::table.count().get_result::<i64>(&mut conn)?;
        Ok(count)
    }

    /// Get an aircraft by its address
    /// Address is unique across all aircraft
    pub async fn get_aircraft_by_address(&self, address: u32) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let model = aircraft::table
            .filter(aircraft::address.eq(address as i32))
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(model.map(|model| model.into()))
    }

    /// Get an aircraft model (with UUID) by address
    pub async fn get_aircraft_model_by_address(
        &self,
        address: i32,
    ) -> Result<Option<AircraftModel>> {
        let mut conn = self.get_connection()?;
        let model = aircraft::table
            .filter(aircraft::address.eq(address))
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;
        Ok(model)
    }

    /// Get or insert an aircraft by address
    /// If the aircraft doesn't exist, it will be created with from_ddb=false, tracked=true, identified=true
    /// Uses INSERT ... ON CONFLICT to handle race conditions atomically
    pub async fn get_or_insert_aircraft_by_address(
        &self,
        address: i32,
        address_type: AddressType,
    ) -> Result<AircraftModel> {
        let mut conn = self.get_connection()?;

        // Extract country code from ICAO address if applicable
        let country_code = Aircraft::extract_country_code_from_icao(address as u32, address_type);

        // Extract tail number from ICAO address if it's a US aircraft
        let registration = Aircraft::extract_tail_number_from_icao(address as u32, address_type)
            .unwrap_or_default();

        let new_aircraft = NewAircraft {
            address,
            address_type,
            aircraft_model: String::new(),
            registration,
            competition_number: String::new(),
            tracked: true,
            identified: true,
            from_ddb: false,
            frequency_mhz: None,
            pilot_name: None,
            home_base_airport_ident: None,
            aircraft_type_ogn: None,
            last_fix_at: None,
            club_id: None,
            icao_model_code: None,
            adsb_emitter_category: None,
            tracker_device_type: None,
            country_code,
            latitude: None,
            longitude: None,
        };

        // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
        // This ensures we always get a aircraft_id, even if concurrent inserts happen
        // The DO UPDATE with a no-op ensures RETURNING gives us the existing row on conflict
        let model = diesel::insert_into(aircraft::table)
            .values(&new_aircraft)
            .on_conflict((aircraft::address_type, aircraft::address))
            .do_update()
            .set(aircraft::address.eq(excluded(aircraft::address))) // No-op update to trigger RETURNING
            .returning(AircraftModel::as_returning())
            .get_result(&mut conn)?;

        Ok(model)
    }

    /// Get or insert an aircraft for fix processing
    /// This method is optimized for the high-frequency fix processing path:
    /// - If aircraft doesn't exist, creates it with all available fields from the packet
    /// - If aircraft exists, atomically updates all packet-derived fields in one operation:
    ///   - last_fix_at (always)
    ///   - aircraft_type_ogn, icao_model_code, adsb_emitter_category, tracker_device_type, registration
    /// - Always returns the aircraft in one atomic operation
    ///
    /// This avoids both no-op updates and separate update tasks for modified fields
    pub async fn aircraft_for_fix(
        &self,
        address: i32,
        address_type: AddressType,
        fix_timestamp: DateTime<Utc>,
        packet_fields: AircraftPacketFields,
        latitude: f64,
        longitude: f64,
    ) -> Result<AircraftModel> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Extract country code from ICAO address if applicable
            let country_code =
                Aircraft::extract_country_code_from_icao(address as u32, address_type);

            // Extract tail number from ICAO address if it's a US aircraft
            // Use packet registration if available and non-empty, otherwise try to extract from ICAO
            let registration = packet_fields
                .registration
                .as_ref()
                .filter(|r| !r.is_empty())
                .cloned()
                .or_else(|| Aircraft::extract_tail_number_from_icao(address as u32, address_type))
                .unwrap_or_default();

            let new_aircraft = NewAircraft {
                address,
                address_type,
                aircraft_model: String::new(),
                registration: registration.clone(),
                competition_number: String::new(),
                tracked: true,
                identified: true,
                from_ddb: false,
                frequency_mhz: None,
                pilot_name: None,
                home_base_airport_ident: None,
                aircraft_type_ogn: packet_fields.aircraft_type,
                last_fix_at: Some(fix_timestamp),
                club_id: None,
                icao_model_code: packet_fields.icao_model_code.clone(),
                adsb_emitter_category: packet_fields.adsb_emitter_category,
                tracker_device_type: packet_fields.tracker_device_type.clone(),
                country_code: country_code.clone(),
                latitude: None,
                longitude: None,
            };

            // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
            // On conflict, update all packet-derived fields atomically in one operation
            // This eliminates the need for separate async update tasks
            let model = diesel::insert_into(aircraft::table)
                .values(&new_aircraft)
                .on_conflict((aircraft::address_type, aircraft::address))
                .do_update()
                .set((
                    aircraft::last_fix_at.eq(fix_timestamp),
                    aircraft::aircraft_type_ogn.eq(packet_fields.aircraft_type),
                    aircraft::icao_model_code.eq(packet_fields.icao_model_code),
                    aircraft::adsb_emitter_category.eq(packet_fields.adsb_emitter_category),
                    aircraft::tracker_device_type.eq(packet_fields.tracker_device_type),
                    aircraft::registration.eq(&registration),
                    aircraft::country_code.eq(&country_code),
                    aircraft::latitude.eq(latitude),
                    aircraft::longitude.eq(longitude),
                ))
                .returning(AircraftModel::as_returning())
                .get_result(&mut conn)?;

            Ok::<AircraftModel, anyhow::Error>(model)
        })
        .await?
    }

    /// Get an aircraft by its ID
    pub async fn get_aircraft_by_id(&self, aircraft_id: Uuid) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let model = aircraft::table
            .filter(aircraft::id.eq(aircraft_id))
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(model.map(|model| model.into()))
    }

    /// Search for all aircraft assigned to a specific club
    pub async fn search_by_club_id(&self, club_id: Uuid) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;

        let models = aircraft::table
            .filter(aircraft::club_id.eq(club_id))
            .order_by(aircraft::registration)
            .select(AircraftModel::as_select())
            .load(&mut conn)?;

        Ok(models.into_iter().map(|model| model.into()).collect())
    }

    /// Search aircraft by address
    /// Returns a single aircraft since address is unique
    pub async fn search_by_address(&self, address: u32) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let model = aircraft::table
            .filter(aircraft::address.eq(address as i32))
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(model.map(|model| model.into()))
    }

    /// Search aircraft by registration
    pub async fn search_by_registration(&self, registration: &str) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;
        let search_pattern = format!("%{}%", registration);
        let models = aircraft::table
            .filter(aircraft::registration.ilike(&search_pattern))
            .select(AircraftModel::as_select())
            .load(&mut conn)?;

        Ok(models.into_iter().map(|model| model.into()).collect())
    }

    /// Get recent aircraft with a limit, ordered by last_fix_at (most recently heard from)
    /// Optionally filter by aircraft types
    pub async fn get_recent_aircraft(
        &self,
        limit: i64,
        aircraft_types: Option<Vec<String>>,
    ) -> Result<Vec<Aircraft>> {
        use diesel::ExpressionMethods;

        let mut conn = self.get_connection()?;

        let mut query = aircraft::table
            .filter(aircraft::last_fix_at.is_not_null())
            .into_boxed();

        // Apply aircraft type filter if provided
        if let Some(types) = aircraft_types
            && !types.is_empty()
        {
            // Convert string aircraft types to AircraftType enum values
            let aircraft_type_enums: Vec<crate::ogn_aprs_aircraft::AircraftType> = types
                .iter()
                .filter_map(|t| match t.as_str() {
                    "glider" => Some(AircraftType::Glider),
                    "tow_tug" => Some(AircraftType::TowTug),
                    "helicopter_gyro" => Some(AircraftType::HelicopterGyro),
                    "skydiver_parachute" => Some(AircraftType::SkydiverParachute),
                    "drop_plane" => Some(AircraftType::DropPlane),
                    "hang_glider" => Some(AircraftType::HangGlider),
                    "paraglider" => Some(AircraftType::Paraglider),
                    "recip_engine" => Some(AircraftType::RecipEngine),
                    "jet_turboprop" => Some(AircraftType::JetTurboprop),
                    "unknown" => Some(AircraftType::Unknown),
                    "balloon" => Some(AircraftType::Balloon),
                    "airship" => Some(AircraftType::Airship),
                    "uav" => Some(AircraftType::Uav),
                    "static_obstacle" => Some(AircraftType::StaticObstacle),
                    "reserved" => Some(AircraftType::Reserved),
                    _ => None,
                })
                .collect();

            if !aircraft_type_enums.is_empty() {
                query = query.filter(aircraft::aircraft_type_ogn.eq_any(aircraft_type_enums));
            }
        }

        let models = query
            .order(aircraft::last_fix_at.desc())
            .limit(limit)
            .select(AircraftModel::as_select())
            .load(&mut conn)?;

        Ok(models.into_iter().map(|model| model.into()).collect())
    }

    /// Get recent aircraft with latest fix location and active flight ID
    /// This extended version includes lat/lng and flight_id for quick navigation
    pub async fn get_recent_aircraft_with_location(
        &self,
        limit: i64,
        aircraft_types: Option<Vec<String>>,
    ) -> Result<Vec<(AircraftModel, Option<f64>, Option<f64>, Option<Uuid>)>> {
        let pool = self.pool.clone();
        let aircraft_types_filter = aircraft_types.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Build aircraft type filter condition
            let aircraft_type_condition = if let Some(types) = aircraft_types_filter
                && !types.is_empty()
            {
                let types_str = types
                    .iter()
                    .map(|t| format!("'{}'", t))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("AND d.aircraft_type_ogn::text IN ({})", types_str)
            } else {
                String::new()
            };

            let query = format!(
                r#"
                SELECT
                    d.*,
                    latest_fix.latitude AS latest_latitude,
                    latest_fix.longitude AS latest_longitude,
                    active_flight.id AS active_flight_id
                FROM aircraft d
                LEFT JOIN LATERAL (
                    SELECT latitude, longitude
                    FROM fixes
                    WHERE aircraft_id = d.id
                    AND received_at >= NOW() - INTERVAL '24 hours'
                    ORDER BY received_at DESC
                    LIMIT 1
                ) latest_fix ON true
                LEFT JOIN flights active_flight ON (
                    active_flight.aircraft_id = d.id
                    AND active_flight.landing_time IS NULL
                    AND active_flight.timed_out_at IS NULL
                )
                WHERE d.last_fix_at IS NOT NULL
                {}
                ORDER BY d.last_fix_at DESC
                LIMIT $1
                "#,
                aircraft_type_condition
            );

            use diesel::sql_query;
            use diesel::sql_types::{
                BigInt, Bool, Float8, Int4, Nullable, Numeric, Text, Timestamptz,
            };

            #[derive(diesel::QueryableByName)]
            struct AircraftWithLocation {
                #[diesel(sql_type = Int4)]
                address: i32,
                #[diesel(sql_type = crate::schema::sql_types::AddressType)]
                address_type: crate::aircraft::AddressType,
                #[diesel(sql_type = Text)]
                aircraft_model: String,
                #[diesel(sql_type = Text)]
                registration: String,
                #[diesel(sql_type = Text)]
                competition_number: String,
                #[diesel(sql_type = Bool)]
                tracked: bool,
                #[diesel(sql_type = Bool)]
                identified: bool,
                #[diesel(sql_type = Timestamptz)]
                created_at: chrono::DateTime<chrono::Utc>,
                #[diesel(sql_type = Timestamptz)]
                updated_at: chrono::DateTime<chrono::Utc>,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: uuid::Uuid,
                #[diesel(sql_type = Bool)]
                from_ddb: bool,
                #[diesel(sql_type = Nullable<Numeric>)]
                frequency_mhz: Option<bigdecimal::BigDecimal>,
                #[diesel(sql_type = Nullable<Text>)]
                pilot_name: Option<String>,
                #[diesel(sql_type = Nullable<Text>)]
                home_base_airport_ident: Option<String>,
                #[diesel(sql_type = Nullable<crate::schema::sql_types::AircraftTypeOgn>)]
                aircraft_type_ogn: Option<crate::ogn_aprs_aircraft::AircraftType>,
                #[diesel(sql_type = Nullable<Timestamptz>)]
                last_fix_at: Option<chrono::DateTime<chrono::Utc>>,
                #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
                club_id: Option<uuid::Uuid>,
                #[diesel(sql_type = Nullable<Text>)]
                icao_model_code: Option<String>,
                #[diesel(sql_type = Nullable<crate::schema::sql_types::AdsbEmitterCategory>)]
                adsb_emitter_category: Option<crate::ogn_aprs_aircraft::AdsbEmitterCategory>,
                #[diesel(sql_type = Nullable<Text>)]
                tracker_device_type: Option<String>,
                #[diesel(sql_type = Nullable<Text>)]
                country_code: Option<String>,
                #[diesel(sql_type = Nullable<Float8>)]
                latest_latitude: Option<f64>,
                #[diesel(sql_type = Nullable<Float8>)]
                latest_longitude: Option<f64>,
                #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
                active_flight_id: Option<uuid::Uuid>,
            }

            let results: Vec<AircraftWithLocation> =
                sql_query(query).bind::<BigInt, _>(limit).load(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|row| {
                    let model = AircraftModel {
                        id: row.id,
                        address: row.address,
                        address_type: row.address_type,
                        aircraft_model: row.aircraft_model,
                        registration: row.registration,
                        competition_number: row.competition_number,
                        tracked: row.tracked,
                        identified: row.identified,
                        created_at: row.created_at,
                        updated_at: row.updated_at,
                        from_ddb: row.from_ddb,
                        frequency_mhz: row.frequency_mhz,
                        pilot_name: row.pilot_name,
                        home_base_airport_ident: row.home_base_airport_ident,
                        aircraft_type_ogn: row.aircraft_type_ogn,
                        last_fix_at: row.last_fix_at,
                        club_id: row.club_id,
                        icao_model_code: row.icao_model_code,
                        adsb_emitter_category: row.adsb_emitter_category,
                        tracker_device_type: row.tracker_device_type,
                        country_code: row.country_code,
                        latitude: None,  // Not selected in this query
                        longitude: None, // Not selected in this query
                    };
                    (
                        model,
                        row.latest_latitude,
                        row.latest_longitude,
                        row.active_flight_id,
                    )
                })
                .collect())
        })
        .await?
    }

    /// Update the club assignment for an aircraft
    pub async fn update_club_id(&self, aircraft_id: Uuid, club_id: Option<Uuid>) -> Result<bool> {
        let mut conn = self.get_connection()?;

        let rows_updated = diesel::update(aircraft::table.filter(aircraft::id.eq(aircraft_id)))
            .set(aircraft::club_id.eq(club_id))
            .execute(&mut conn)?;

        if rows_updated > 0 {
            info!(
                "Updated aircraft {} club assignment to {:?}",
                aircraft_id, club_id
            );
            Ok(true)
        } else {
            warn!("No aircraft found with ID {}", aircraft_id);
            Ok(false)
        }
    }

    /// Query for aircraft that have duplicate addresses (same address, different address_type)
    pub async fn get_duplicate_aircraft(&self) -> Result<Vec<AircraftModel>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Find addresses that appear more than once with different address types
            // Use a subquery to get addresses where COUNT(DISTINCT address_type) > 1
            let duplicate_addresses: Vec<i32> = aircraft::table
                .select(aircraft::address)
                .group_by(aircraft::address)
                .having(diesel::dsl::sql::<diesel::sql_types::Bool>(
                    "COUNT(DISTINCT address_type) > 1",
                ))
                .load(&mut conn)?;

            if duplicate_addresses.is_empty() {
                return Ok(Vec::new());
            }

            // Now fetch all aircraft rows for those duplicate addresses
            let duplicate_aircraft = aircraft::table
                .filter(aircraft::address.eq_any(duplicate_addresses))
                .order((aircraft::address.asc(), aircraft::address_type.asc()))
                .select(AircraftModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<AircraftModel>, anyhow::Error>(duplicate_aircraft)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::r2d2::ConnectionManager;

    // Helper function to create a test database pool (for integration tests)
    fn create_test_pool() -> Result<PgPool> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/soar_test".to_string());

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder().build(manager)?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_aircraft_repository_creation() {
        // Just test that we can create the repository
        if let Ok(pool) = create_test_pool() {
            let _repo = AircraftRepository::new(pool);
        } else {
            // Skip test if we can't connect to test database
            println!("Skipping test - no test database connection");
        }
    }
}
