use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use tracing::debug;
use uuid::Uuid;

use crate::fixes::Fix;
use crate::ogn_aprs_aircraft::{
    AddressType as ForeignAddressType, AdsbEmitterCategory, AircraftType as ForeignAircraftType,
};
use crate::web::PgPool;

// Import the main AddressType from devices module
use crate::devices::AddressType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum)]
#[db_enum(existing_type_path = "crate::schema::sql_types::AircraftTypeOgn")]
pub enum AircraftTypeOgn {
    Reserved,
    Glider,
    TowTug,
    HelicopterGyro,
    SkydiverParachute,
    DropPlane,
    HangGlider,
    Paraglider,
    RecipEngine,
    JetTurboprop,
    Unknown,
    Balloon,
    Airship,
    Uav,
    StaticObstacle,
}

// Conversions between foreign types and wrapper types
impl From<ForeignAddressType> for AddressType {
    fn from(foreign_type: ForeignAddressType) -> Self {
        match foreign_type {
            ForeignAddressType::Unknown => AddressType::Unknown,
            ForeignAddressType::Icao => AddressType::Icao,
            ForeignAddressType::Flarm => AddressType::Flarm,
            ForeignAddressType::OgnTracker => AddressType::Ogn,
        }
    }
}

impl From<AddressType> for ForeignAddressType {
    fn from(wrapper_type: AddressType) -> Self {
        match wrapper_type {
            AddressType::Unknown => ForeignAddressType::Unknown,
            AddressType::Icao => ForeignAddressType::Icao,
            AddressType::Flarm => ForeignAddressType::Flarm,
            AddressType::Ogn => ForeignAddressType::OgnTracker,
        }
    }
}

impl From<ForeignAircraftType> for AircraftTypeOgn {
    fn from(foreign_type: ForeignAircraftType) -> Self {
        match foreign_type {
            ForeignAircraftType::Reserved => AircraftTypeOgn::Reserved,
            ForeignAircraftType::Glider => AircraftTypeOgn::Glider,
            ForeignAircraftType::TowTug => AircraftTypeOgn::TowTug,
            ForeignAircraftType::HelicopterGyro => AircraftTypeOgn::HelicopterGyro,
            ForeignAircraftType::SkydiverParachute => AircraftTypeOgn::SkydiverParachute,
            ForeignAircraftType::DropPlane => AircraftTypeOgn::DropPlane,
            ForeignAircraftType::HangGlider => AircraftTypeOgn::HangGlider,
            ForeignAircraftType::Paraglider => AircraftTypeOgn::Paraglider,
            ForeignAircraftType::RecipEngine => AircraftTypeOgn::RecipEngine,
            ForeignAircraftType::JetTurboprop => AircraftTypeOgn::JetTurboprop,
            ForeignAircraftType::Unknown => AircraftTypeOgn::Unknown,
            ForeignAircraftType::Balloon => AircraftTypeOgn::Balloon,
            ForeignAircraftType::Airship => AircraftTypeOgn::Airship,
            ForeignAircraftType::Uav => AircraftTypeOgn::Uav,
            ForeignAircraftType::StaticObstacle => AircraftTypeOgn::StaticObstacle,
        }
    }
}

impl From<AircraftTypeOgn> for ForeignAircraftType {
    fn from(wrapper_type: AircraftTypeOgn) -> Self {
        match wrapper_type {
            AircraftTypeOgn::Reserved => ForeignAircraftType::Reserved,
            AircraftTypeOgn::Glider => ForeignAircraftType::Glider,
            AircraftTypeOgn::TowTug => ForeignAircraftType::TowTug,
            AircraftTypeOgn::HelicopterGyro => ForeignAircraftType::HelicopterGyro,
            AircraftTypeOgn::SkydiverParachute => ForeignAircraftType::SkydiverParachute,
            AircraftTypeOgn::DropPlane => ForeignAircraftType::DropPlane,
            AircraftTypeOgn::HangGlider => ForeignAircraftType::HangGlider,
            AircraftTypeOgn::Paraglider => ForeignAircraftType::Paraglider,
            AircraftTypeOgn::RecipEngine => ForeignAircraftType::RecipEngine,
            AircraftTypeOgn::JetTurboprop => ForeignAircraftType::JetTurboprop,
            AircraftTypeOgn::Unknown => ForeignAircraftType::Unknown,
            AircraftTypeOgn::Balloon => ForeignAircraftType::Balloon,
            AircraftTypeOgn::Airship => ForeignAircraftType::Airship,
            AircraftTypeOgn::Uav => ForeignAircraftType::Uav,
            AircraftTypeOgn::StaticObstacle => ForeignAircraftType::StaticObstacle,
        }
    }
}

// Queryable struct for Diesel DSL queries (excluding geography column)
#[derive(Queryable, Debug)]
struct FixDslRow {
    id: Uuid,
    source: String,
    destination: String,
    via: Vec<Option<String>>, // NOT NULL array that can contain NULL elements
    raw_packet: String,
    timestamp: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    altitude_feet: Option<i32>,
    device_address: i32,
    address_type: AddressType,
    aircraft_type_ogn: Option<AircraftTypeOgn>,
    flight_number: Option<String>,
    emitter_category: Option<AdsbEmitterCategory>,
    registration: Option<String>,
    model: Option<String>,
    squawk: Option<String>,
    ground_speed_knots: Option<f32>,
    track_degrees: Option<f32>,
    climb_fpm: Option<i32>,
    turn_rate_rot: Option<f32>,
    snr_db: Option<f32>,
    bit_errors_corrected: Option<i32>,
    freq_offset_khz: Option<f32>,
    club_id: Option<Uuid>,
    flight_id: Option<Uuid>,
    unparsed_data: Option<String>,
    device_id: Uuid,
    received_at: DateTime<Utc>,
    lag: Option<i32>,
}

impl From<FixDslRow> for Fix {
    fn from(row: FixDslRow) -> Self {
        Self {
            id: row.id,
            source: row.source,
            destination: row.destination,
            via: row.via, // Now directly a Vec<Option<String>>
            raw_packet: row.raw_packet,
            timestamp: row.timestamp,
            received_at: row.received_at,
            lag: row.lag,
            latitude: row.latitude,
            longitude: row.longitude,
            altitude_feet: row.altitude_feet,
            device_address: row.device_address,
            address_type: row.address_type,
            aircraft_type_ogn: row.aircraft_type_ogn.map(|t| t.into()),
            flight_id: row.flight_id,
            flight_number: row.flight_number,
            emitter_category: row.emitter_category,
            registration: row.registration,
            model: row.model,
            squawk: row.squawk,
            ground_speed_knots: row.ground_speed_knots,
            track_degrees: row.track_degrees,
            climb_fpm: row.climb_fpm,
            turn_rate_rot: row.turn_rate_rot,
            snr_db: row.snr_db,
            bit_errors_corrected: row.bit_errors_corrected,
            freq_offset_khz: row.freq_offset_khz,
            club_id: row.club_id,
            unparsed_data: row.unparsed_data,
            device_id: row.device_id, // Now directly a Uuid
        }
    }
}

#[derive(Clone)]
pub struct FixesRepository {
    pool: PgPool,
}

impl FixesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Look up device UUID by device_address (hex string) and address_type
    fn lookup_device_uuid_by_address(
        conn: &mut diesel::PgConnection,
        device_address: &str,
        address_type_: AddressType,
    ) -> Result<Uuid> {
        // Convert hex string to integer for database lookup
        let device_address_int = u32::from_str_radix(device_address, 16)?;
        let opt_uuid = Self::lookup_device_uuid(conn, device_address_int, address_type_)?;
        opt_uuid.ok_or_else(|| anyhow::anyhow!("Device not found"))
    }

    /// Look up device UUID by raw device_id and address_type (legacy method)
    fn lookup_device_uuid(
        conn: &mut diesel::PgConnection,
        raw_device_id: u32,
        address_type_: AddressType,
    ) -> Result<Option<Uuid>> {
        use crate::schema::devices::dsl::*;

        let device_uuid = devices
            .filter(address.eq(raw_device_id as i32))
            .filter(address_type.eq(address_type_))
            .select(id)
            .first::<Uuid>(conn)
            .optional()?;

        Ok(device_uuid)
    }

    /// Insert a new fix into the database
    pub async fn insert(&self, fix: &Fix) -> Result<()> {
        use crate::schema::fixes::dsl::*;

        let mut new_fix = fix.clone();
        let pool = self.pool.clone();

        // Look up device UUID if we have device address and address type
        let dev_address = fix.device_address_hex();
        let address_type_enum = fix.address_type;
        let dev_address_owned = dev_address.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Look up the device UUID using device address and address type
            new_fix.device_id = Self::lookup_device_uuid_by_address(
                &mut conn,
                    &dev_address_owned,
                    address_type_enum,
                )?;

            diesel::insert_into(fixes)
                .values(&new_fix)
                .execute(&mut conn)?;

            debug!(
                "Inserted fix | Device: {:?} ({:?}-{:?}) | {:.6},{:.6} @ {}ft | https://maps.google.com/maps?q={:.6},{:.6}",
                new_fix.device_id,
                new_fix.address_type,
                new_fix.device_address,
                new_fix.latitude,
                new_fix.longitude,
                new_fix.altitude_feet.map_or("Unknown".to_string(), |a| a.to_string()),
                new_fix.latitude,
                new_fix.longitude
            );

                Ok::<(), anyhow::Error>(())
            })
            .await?
    }

    /// Get fixes for a specific aircraft ID within a time range (original method)
    pub async fn get_fixes_for_aircraft_with_time_range(
        &self,
        device_id: &uuid::Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);
        let device_id_param = *device_id;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let results = fixes
                .filter(device_id.eq(device_id_param))
                .filter(timestamp.between(start_time, end_time))
                .order(timestamp.desc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get recent fixes for an aircraft (without time range)
    pub async fn get_fixes_for_device(
        &self,
        device_uuid: uuid::Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        use crate::schema::fixes::dsl::*;

        let limit = limit.unwrap_or(100);
        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let results = fixes
                .filter(device_id.eq(device_uuid))
                .order(timestamp.desc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get recent fixes (most recent first)
    pub async fn get_recent_fixes(&self, limit: i64) -> Result<Vec<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let results = fixes
                .order(timestamp.desc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Get fixes for a specific device with optional after timestamp and limit
    pub async fn get_fixes_by_device(
        &self,
        device_uuid: Uuid,
        after: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<Fix>> {
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;
            let mut query = fixes.filter(device_id.eq(device_uuid)).into_boxed();
            if let Some(after_timestamp) = after {
                query = query.filter(timestamp.gt(after_timestamp));
            }
            let results = query
                .order(timestamp.asc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;
            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }

    /// Update flight_id for fixes by device_address within a time range
    /// This is used by flight detection processor to link fixes to flights after they're created
    pub async fn update_flight_id_by_device_and_time(
        &self,
        device_id: Uuid,
        flight_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<usize, anyhow::Error> {
        let pool = self.pool.clone();
        let device_id_param = device_id;
        let flight_id_param = flight_id;

        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let updated_count = if let Some(end_time) = end_time {
                diesel::update(fixes)
                    .filter(device_id.eq(device_id_param))
                    .filter(timestamp.ge(start_time))
                    .filter(timestamp.le(end_time))
                    .filter(flight_id.is_null())
                    .set(flight_id.eq(flight_id_param))
                    .execute(&mut conn)?
            } else {
                diesel::update(fixes)
                    .filter(device_address.eq(&device_address))
                    .filter(timestamp.ge(start_time))
                    .filter(flight_id.is_null())
                    .set(flight_id.eq(flight_id_param))
                    .execute(&mut conn)?
            };

            Ok::<usize, anyhow::Error>(updated_count)
        })
        .await??;

        debug!(
            "Updated {} fixes with flight_id {} for device {}",
            result, flight_id_param, device_id_param
        );

        Ok(result)
    }

    /// Get fixes for a specific flight ID
    pub async fn get_fixes_for_flight(
        &self,
        flight_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Fix>> {
        let limit = limit.unwrap_or(1000);
        let flight_id_param = flight_id;
        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use crate::schema::fixes::dsl::*;
            let mut conn = pool.get()?;

            let results = fixes
                .filter(flight_id.eq(flight_id_param))
                .order(timestamp.asc())
                .limit(limit)
                .select(Fix::as_select())
                .load::<Fix>(&mut conn)?;

            Ok::<Vec<Fix>, anyhow::Error>(results)
        })
        .await??;

        Ok(result)
    }
}
