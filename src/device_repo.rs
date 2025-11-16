use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use tracing::{info, warn};
use uuid::Uuid;

use crate::devices::{AddressType, Device, DeviceModel, NewDevice};
use crate::ogn_aprs_aircraft::{AdsbEmitterCategory, AircraftType};
use crate::schema::devices;
use chrono::{DateTime, Utc};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Fields extracted from packet for device creation/update
#[derive(Debug, Clone)]
pub struct DevicePacketFields {
    pub aircraft_type: Option<AircraftType>,
    pub icao_model_code: Option<String>,
    pub adsb_emitter_category: Option<AdsbEmitterCategory>,
    pub tracker_device_type: Option<String>,
    pub registration: Option<String>,
}

#[derive(Clone)]
pub struct DeviceRepository {
    pool: PgPool,
}

impl DeviceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn get_connection(&self) -> Result<PgPooledConnection> {
        self.pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))
    }

    /// Upsert devices into the database
    /// This will insert new devices or update existing ones based on device_id
    pub async fn upsert_devices<I>(&self, devices_iter: I) -> Result<usize>
    where
        I: IntoIterator<Item = Device>,
    {
        let mut conn = self.get_connection()?;
        let mut upserted_count = 0;

        // Convert devices to NewDevice structs for insertion
        let new_devices: Vec<NewDevice> = devices_iter.into_iter().map(|d| d.into()).collect();

        for new_device in new_devices {
            let result = diesel::insert_into(devices::table)
                .values(&new_device)
                .on_conflict((devices::address_type, devices::address))
                .do_update()
                .set((
                    // Update fields from DDB, but preserve existing values if DDB value is empty
                    // Use COALESCE(NULLIF(new, ''), old) to keep existing data when DDB has empty strings
                    devices::address_type.eq(excluded(devices::address_type)),
                    devices::aircraft_model.eq(diesel::dsl::sql(
                        "COALESCE(NULLIF(EXCLUDED.aircraft_model, ''), devices.aircraft_model)"
                    )),
                    devices::registration.eq(diesel::dsl::sql(
                        "COALESCE(NULLIF(EXCLUDED.registration, ''), devices.registration)"
                    )),
                    devices::competition_number.eq(diesel::dsl::sql(
                        "COALESCE(NULLIF(EXCLUDED.competition_number, ''), devices.competition_number)"
                    )),
                    devices::tracked.eq(excluded(devices::tracked)),
                    devices::identified.eq(excluded(devices::identified)),
                    devices::from_ddb.eq(excluded(devices::from_ddb)),
                    // For Option fields, use COALESCE to prefer new value over NULL, but keep old if new is NULL
                    devices::frequency_mhz.eq(diesel::dsl::sql(
                        "COALESCE(EXCLUDED.frequency_mhz, devices.frequency_mhz)"
                    )),
                    devices::pilot_name.eq(diesel::dsl::sql(
                        "COALESCE(EXCLUDED.pilot_name, devices.pilot_name)"
                    )),
                    devices::home_base_airport_ident.eq(diesel::dsl::sql(
                        "COALESCE(EXCLUDED.home_base_airport_ident, devices.home_base_airport_ident)"
                    )),
                    devices::updated_at.eq(diesel::dsl::now),
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
                    warn!("Failed to upsert device {}: {}", new_device.address, e);
                    // Continue with other devices rather than failing the entire batch
                }
            }
        }

        info!("Successfully upserted {} devices", upserted_count);
        Ok(upserted_count)
    }

    /// Get the total count of devices in the database
    pub async fn get_device_count(&self) -> Result<i64> {
        let mut conn = self.get_connection()?;
        let count = devices::table.count().get_result::<i64>(&mut conn)?;
        Ok(count)
    }

    /// Get a device by its address
    /// Address is unique across all devices
    pub async fn get_device_by_address(&self, address: u32) -> Result<Option<Device>> {
        let mut conn = self.get_connection()?;
        let device_model = devices::table
            .filter(devices::address.eq(address as i32))
            .first::<DeviceModel>(&mut conn)
            .optional()?;

        Ok(device_model.map(|model| model.into()))
    }

    /// Get a device model (with UUID) by address
    pub async fn get_device_model_by_address(&self, address: i32) -> Result<Option<DeviceModel>> {
        let mut conn = self.get_connection()?;
        let device_model = devices::table
            .filter(devices::address.eq(address))
            .first::<DeviceModel>(&mut conn)
            .optional()?;
        Ok(device_model)
    }

    /// Get or insert a device by address
    /// If the device doesn't exist, it will be created with from_ddb=false, tracked=true, identified=true
    /// Uses INSERT ... ON CONFLICT to handle race conditions atomically
    pub async fn get_or_insert_device_by_address(
        &self,
        address: i32,
        address_type: AddressType,
    ) -> Result<DeviceModel> {
        let mut conn = self.get_connection()?;

        // Extract country code from ICAO address if applicable
        let country_code = Device::extract_country_code_from_icao(address as u32, address_type);

        // Extract tail number from ICAO address if it's a US aircraft
        let registration =
            Device::extract_tail_number_from_icao(address as u32, address_type).unwrap_or_default();

        let new_device = NewDevice {
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
        };

        // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
        // This ensures we always get a device_id, even if concurrent inserts happen
        // The DO UPDATE with a no-op ensures RETURNING gives us the existing row on conflict
        let device_model = diesel::insert_into(devices::table)
            .values(&new_device)
            .on_conflict((devices::address_type, devices::address))
            .do_update()
            .set(devices::address.eq(excluded(devices::address))) // No-op update to trigger RETURNING
            .get_result::<DeviceModel>(&mut conn)?;

        Ok(device_model)
    }

    /// Get or insert a device for fix processing
    /// This method is optimized for the high-frequency fix processing path:
    /// - If device doesn't exist, creates it with all available fields from the packet
    /// - If device exists, atomically updates all packet-derived fields in one operation:
    ///   - last_fix_at (always)
    ///   - aircraft_type_ogn, icao_model_code, adsb_emitter_category, tracker_device_type, registration
    /// - Always returns the device in one atomic operation
    ///
    /// This avoids both no-op updates and separate update tasks for modified fields
    pub async fn device_for_fix(
        &self,
        address: i32,
        address_type: AddressType,
        fix_timestamp: DateTime<Utc>,
        packet_fields: DevicePacketFields,
    ) -> Result<DeviceModel> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Extract country code from ICAO address if applicable
            let country_code = Device::extract_country_code_from_icao(address as u32, address_type);

            // Extract tail number from ICAO address if it's a US aircraft
            // Use packet registration if available and non-empty, otherwise try to extract from ICAO
            let registration = packet_fields
                .registration
                .as_ref()
                .filter(|r| !r.is_empty())
                .cloned()
                .or_else(|| Device::extract_tail_number_from_icao(address as u32, address_type))
                .unwrap_or_default();

            let new_device = NewDevice {
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
            };

            // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
            // On conflict, update all packet-derived fields atomically in one operation
            // This eliminates the need for separate async update tasks
            let device_model = diesel::insert_into(devices::table)
                .values(&new_device)
                .on_conflict((devices::address_type, devices::address))
                .do_update()
                .set((
                    devices::last_fix_at.eq(fix_timestamp),
                    devices::aircraft_type_ogn.eq(packet_fields.aircraft_type),
                    devices::icao_model_code.eq(packet_fields.icao_model_code),
                    devices::adsb_emitter_category.eq(packet_fields.adsb_emitter_category),
                    devices::tracker_device_type.eq(packet_fields.tracker_device_type),
                    devices::registration.eq(&registration),
                    devices::country_code.eq(&country_code),
                ))
                .get_result::<DeviceModel>(&mut conn)?;

            Ok::<DeviceModel, anyhow::Error>(device_model)
        })
        .await?
    }

    /// Get a device by its UUID
    pub async fn get_device_by_uuid(&self, device_uuid: Uuid) -> Result<Option<Device>> {
        let mut conn = self.get_connection()?;
        let device_model = devices::table
            .filter(devices::id.eq(device_uuid))
            .first::<DeviceModel>(&mut conn)
            .optional()?;

        Ok(device_model.map(|model| model.into()))
    }

    /// Search for all devices (aircraft) assigned to a specific club
    pub async fn search_by_club_id(&self, club_id: Uuid) -> Result<Vec<Device>> {
        let mut conn = self.get_connection()?;

        let device_models = devices::table
            .filter(devices::club_id.eq(club_id))
            .order_by(devices::registration)
            .load::<DeviceModel>(&mut conn)?;

        Ok(device_models
            .into_iter()
            .map(|model| model.into())
            .collect())
    }

    /// Search devices by address
    /// Returns a single device since address is unique
    pub async fn search_by_address(&self, address: u32) -> Result<Option<Device>> {
        let mut conn = self.get_connection()?;
        let device_model = devices::table
            .filter(devices::address.eq(address as i32))
            .first::<DeviceModel>(&mut conn)
            .optional()?;

        Ok(device_model.map(|model| model.into()))
    }

    /// Search devices by registration
    pub async fn search_by_registration(&self, registration: &str) -> Result<Vec<Device>> {
        let mut conn = self.get_connection()?;
        let search_pattern = format!("%{}%", registration);
        let device_models = devices::table
            .filter(devices::registration.ilike(&search_pattern))
            .load::<DeviceModel>(&mut conn)?;

        Ok(device_models
            .into_iter()
            .map(|model| model.into())
            .collect())
    }

    /// Get recent devices with a limit, ordered by last_fix_at (most recently heard from)
    /// Optionally filter by aircraft types
    pub async fn get_recent_devices(
        &self,
        limit: i64,
        aircraft_types: Option<Vec<String>>,
    ) -> Result<Vec<Device>> {
        use diesel::ExpressionMethods;

        let mut conn = self.get_connection()?;

        let mut query = devices::table
            .filter(devices::last_fix_at.is_not_null())
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
                query = query.filter(devices::aircraft_type_ogn.eq_any(aircraft_type_enums));
            }
        }

        let device_models = query
            .order(devices::last_fix_at.desc())
            .limit(limit)
            .load::<DeviceModel>(&mut conn)?;

        Ok(device_models
            .into_iter()
            .map(|model| model.into())
            .collect())
    }

    /// Get recent devices with latest fix location and active flight ID
    /// This extended version includes lat/lng and flight_id for quick navigation
    pub async fn get_recent_devices_with_location(
        &self,
        limit: i64,
        aircraft_types: Option<Vec<String>>,
    ) -> Result<Vec<(DeviceModel, Option<f64>, Option<f64>, Option<Uuid>)>> {
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
                FROM devices d
                LEFT JOIN LATERAL (
                    SELECT latitude, longitude
                    FROM fixes
                    WHERE device_id = d.id
                    AND received_at >= NOW() - INTERVAL '24 hours'
                    ORDER BY received_at DESC
                    LIMIT 1
                ) latest_fix ON true
                LEFT JOIN flights active_flight ON (
                    active_flight.device_id = d.id
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
            struct DeviceWithLocation {
                #[diesel(sql_type = Int4)]
                address: i32,
                #[diesel(sql_type = crate::schema::sql_types::AddressType)]
                address_type: crate::devices::AddressType,
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

            let results: Vec<DeviceWithLocation> =
                sql_query(query).bind::<BigInt, _>(limit).load(&mut conn)?;

            Ok(results
                .into_iter()
                .map(|row| {
                    let device_model = DeviceModel {
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
                    };
                    (
                        device_model,
                        row.latest_latitude,
                        row.latest_longitude,
                        row.active_flight_id,
                    )
                })
                .collect())
        })
        .await?
    }

    /// Update the club assignment for a device
    pub async fn update_club_id(&self, device_id: Uuid, club_id: Option<Uuid>) -> Result<bool> {
        let mut conn = self.get_connection()?;

        let rows_updated = diesel::update(devices::table.filter(devices::id.eq(device_id)))
            .set(devices::club_id.eq(club_id))
            .execute(&mut conn)?;

        if rows_updated > 0 {
            info!(
                "Updated device {} club assignment to {:?}",
                device_id, club_id
            );
            Ok(true)
        } else {
            warn!("No device found with ID {}", device_id);
            Ok(false)
        }
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
    async fn test_device_repository_creation() {
        // Just test that we can create the repository
        if let Ok(pool) = create_test_pool() {
            let _repo = DeviceRepository::new(pool);
        } else {
            // Skip test if we can't connect to test database
            println!("Skipping test - no test database connection");
        }
    }
}
