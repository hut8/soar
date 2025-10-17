use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use tracing::{info, warn};
use uuid::Uuid;

use crate::devices::{AddressType, Device, DeviceModel, NewDevice};
use crate::ogn_aprs_aircraft::AircraftType;
use crate::schema::devices;
use chrono::{DateTime, Utc};

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

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
                .on_conflict(devices::address)
                .do_update()
                .set((
                    devices::aircraft_model.eq(excluded(devices::aircraft_model)),
                    devices::registration.eq(excluded(devices::registration)),
                    devices::competition_number.eq(excluded(devices::competition_number)),
                    devices::tracked.eq(excluded(devices::tracked)),
                    devices::identified.eq(excluded(devices::identified)),
                    devices::from_ddb.eq(excluded(devices::from_ddb)),
                    devices::updated_at.eq(diesel::dsl::now),
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

        let new_device = NewDevice {
            address,
            address_type,
            aircraft_model: String::new(),
            registration: String::new(),
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
        };

        // Use INSERT ... ON CONFLICT ... DO UPDATE RETURNING to atomically handle race conditions
        // This ensures we always get a device_id, even if concurrent inserts happen
        // The DO UPDATE with a no-op ensures RETURNING gives us the existing row on conflict
        let device_model = diesel::insert_into(devices::table)
            .values(&new_device)
            .on_conflict(devices::address)
            .do_update()
            .set(devices::address.eq(excluded(devices::address))) // No-op update to trigger RETURNING
            .get_result::<DeviceModel>(&mut conn)?;

        Ok(device_model)
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

    /// Get recent devices with a limit
    pub async fn get_recent_devices(&self, limit: i64) -> Result<Vec<Device>> {
        let mut conn = self.get_connection()?;
        let device_models = devices::table
            .order(devices::updated_at.desc().nulls_last())
            .limit(limit)
            .load::<DeviceModel>(&mut conn)?;

        Ok(device_models
            .into_iter()
            .map(|model| model.into())
            .collect())
    }

    /// Update cached fields (aircraft_type_ogn and last_fix_at) from a fix
    pub async fn update_cached_fields(
        &self,
        device_id: Uuid,
        aircraft_type: Option<AircraftType>,
        fix_timestamp: DateTime<Utc>,
    ) -> Result<()> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::update(devices::table.filter(devices::id.eq(device_id)))
                .set((
                    devices::aircraft_type_ogn.eq(aircraft_type),
                    devices::last_fix_at.eq(fix_timestamp),
                ))
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
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
