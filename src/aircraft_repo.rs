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
    pub aircraft_model: Option<String>,
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
                    aircraft::from_ogn_ddb.eq(excluded(aircraft::from_ogn_ddb)),
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
                    aircraft::country_code.eq(diesel::dsl::sql(
                        "COALESCE(EXCLUDED.country_code, aircraft.country_code)"
                    )),
                    aircraft::updated_at.eq(diesel::dsl::now),
                    // NOTE: We do NOT update the following fields because they come from real-time packets:
                    // - aircraft_type_ogn (from OGN packets)
                    // - icao_model_code (from ADSB packets)
                    // - adsb_emitter_category (from ADSB packets)
                    // - tracker_device_type (from tracker packets)
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
    /// If the aircraft doesn't exist, it will be created with from_ogn_ddb=false, tracked=true, identified=true
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
        let registration = Aircraft::extract_tail_number_from_icao(address as u32, address_type);

        let new_aircraft = NewAircraft {
            address,
            address_type,
            aircraft_model: String::new(),
            registration,
            competition_number: String::new(),
            tracked: true,
            identified: true,
            from_ogn_ddb: false,
            from_adsbx_ddb: false,
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
                aircraft_model: packet_fields.aircraft_model.clone().unwrap_or_default(),
                registration: if registration.is_empty() { None } else { Some(registration.clone()) },
                competition_number: String::new(),
                tracked: true,
                identified: true,
                from_ogn_ddb: false,
                from_adsbx_ddb: false,
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
                owner_operator: None,
                aircraft_category: None,
                engine_count: None,
                engine_type: None,
                faa_pia: None,
                faa_ladd: None,
                year: None,
                is_military: None,
                current_fix: None, // Will be populated when fix is inserted
                images: None,
            };

            // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
            // On conflict, update all packet-derived fields atomically in one operation
            // This eliminates the need for separate async update tasks

            // Prepare registration SQL expression
            let registration_sql = if !registration.is_empty() {
                format!("'{}'::text", registration.replace('\'', "''"))
            } else {
                "aircraft.registration".to_string()
            };

            let model = diesel::insert_into(aircraft::table)
                .values(&new_aircraft)
                .on_conflict((aircraft::address_type, aircraft::address))
                .do_update()
                .set((
                    aircraft::last_fix_at.eq(fix_timestamp),
                    aircraft::aircraft_type_ogn.eq(packet_fields.aircraft_type),
                    // Only update icao_model_code if current value is NULL (preserve data from authoritative sources)
                    aircraft::icao_model_code.eq(diesel::dsl::sql("COALESCE(aircraft.icao_model_code, excluded.icao_model_code)")),
                    // Only update adsb_emitter_category if current value is NULL (preserve data from authoritative sources)
                    aircraft::adsb_emitter_category.eq(diesel::dsl::sql("COALESCE(aircraft.adsb_emitter_category, excluded.adsb_emitter_category)")),
                    aircraft::tracker_device_type.eq(packet_fields.tracker_device_type),
                    // Only update aircraft_model if current value is NULL or empty string
                    aircraft::aircraft_model.eq(diesel::dsl::sql(
                        "CASE WHEN (aircraft.aircraft_model IS NULL OR aircraft.aircraft_model = '') \
                         THEN excluded.aircraft_model \
                         ELSE aircraft.aircraft_model END"
                    )),
                    aircraft::registration.eq(diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Text>>(&registration_sql)),
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

    /// Find all aircraft within a bounding box that have recent fixes
    /// Returns aircraft models with all fields populated from the database
    pub async fn find_aircraft_in_bounding_box(
        &self,
        north: f64,
        west: f64,
        south: f64,
        east: f64,
        cutoff_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<AircraftModel>> {
        use diesel::sql_types::{BigInt, Double, Timestamptz};

        let mut conn = self.get_connection()?;

        // Build the SQL query with optional LIMIT clause
        let limit_clause = if limit.is_some() { "LIMIT $6" } else { "" };

        let aircraft_sql = format!(
            r#"
            WITH params AS (
                SELECT
                    $1::double precision AS left_lng,
                    $2::double precision AS bottom_lat,
                    $3::double precision AS right_lng,
                    $4::double precision AS top_lat,
                    $5::timestamptz AS cutoff_time
            ),
            parts AS (
                SELECT
                    CASE WHEN left_lng <= right_lng THEN
                        ARRAY[
                            ST_MakeEnvelope(left_lng, bottom_lat, right_lng, top_lat, 4326)::geometry
                        ]
                    ELSE
                        ARRAY[
                            ST_MakeEnvelope(left_lng, bottom_lat, 180, top_lat, 4326)::geometry,
                            ST_MakeEnvelope(-180, bottom_lat, right_lng, top_lat, 4326)::geometry
                        ]
                    END AS boxes,
                    cutoff_time
                FROM params
            )
            SELECT d.*
            FROM aircraft d, parts
            WHERE d.last_fix_at >= parts.cutoff_time
              AND d.location_geom IS NOT NULL
              AND (
                  d.location_geom && parts.boxes[1]
                  OR (array_length(parts.boxes, 1) = 2 AND d.location_geom && parts.boxes[2])
              )
            {}
        "#,
            limit_clause
        );

        let query = diesel::sql_query(aircraft_sql)
            .bind::<Double, _>(west)
            .bind::<Double, _>(south)
            .bind::<Double, _>(east)
            .bind::<Double, _>(north)
            .bind::<Timestamptz, _>(cutoff_time);

        // Bind limit if provided
        let aircraft_models = if let Some(lim) = limit {
            query
                .bind::<BigInt, _>(lim)
                .load::<AircraftModel>(&mut conn)?
        } else {
            query.load::<AircraftModel>(&mut conn)?
        };

        Ok(aircraft_models)
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

    /// Get aircraft by ID (returns AircraftModel with all fields including images)
    pub async fn get_aircraft_model_by_id(
        &self,
        aircraft_id: Uuid,
    ) -> Result<Option<AircraftModel>> {
        let mut conn = self.get_connection()?;

        let model = aircraft::table
            .filter(aircraft::id.eq(aircraft_id))
            .select(AircraftModel::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(model)
    }

    /// Update the images cache for an aircraft
    pub async fn update_images(
        &self,
        aircraft_id: Uuid,
        images_json: serde_json::Value,
    ) -> Result<bool> {
        let mut conn = self.get_connection()?;

        let rows_updated = diesel::update(aircraft::table.filter(aircraft::id.eq(aircraft_id)))
            .set(aircraft::images.eq(images_json))
            .execute(&mut conn)?;

        if rows_updated > 0 {
            info!("Updated aircraft {} images cache", aircraft_id);
            Ok(true)
        } else {
            warn!("No aircraft found with ID {}", aircraft_id);
            Ok(false)
        }
    }

    /// Query for aircraft that have duplicate addresses with pagination
    /// Returns (results, total_count)
    pub async fn get_duplicate_aircraft_paginated(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<AircraftModel>, i64)> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Find addresses that appear more than once with different address types
            let duplicate_addresses: Vec<i32> = aircraft::table
                .select(aircraft::address)
                .group_by(aircraft::address)
                .having(diesel::dsl::sql::<diesel::sql_types::Bool>(
                    "COUNT(DISTINCT address_type) > 1",
                ))
                .load(&mut conn)?;

            if duplicate_addresses.is_empty() {
                return Ok((Vec::new(), 0));
            }

            // Get total count of aircraft with duplicate addresses
            let total_count = aircraft::table
                .filter(aircraft::address.eq_any(duplicate_addresses.clone()))
                .count()
                .get_result::<i64>(&mut conn)?;

            // Calculate offset
            let offset = (page - 1) * per_page;

            // Fetch paginated results
            let duplicate_aircraft = aircraft::table
                .filter(aircraft::address.eq_any(duplicate_addresses))
                .order((aircraft::address.asc(), aircraft::address_type.asc()))
                .limit(per_page)
                .offset(offset)
                .select(AircraftModel::as_select())
                .load(&mut conn)?;

            Ok::<(Vec<AircraftModel>, i64), anyhow::Error>((duplicate_aircraft, total_count))
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
