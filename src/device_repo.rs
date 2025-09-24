use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use tracing::{info, warn};
use uuid::Uuid;

use crate::devices::{AddressType, Device, DeviceModel, NewDevice};
use crate::schema::devices;

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
                .on_conflict((devices::address_type, devices::address))
                .do_update()
                .set((
                    devices::aircraft_model.eq(excluded(devices::aircraft_model)),
                    devices::registration.eq(excluded(devices::registration)),
                    devices::competition_number.eq(excluded(devices::competition_number)),
                    devices::tracked.eq(excluded(devices::tracked)),
                    devices::identified.eq(excluded(devices::identified)),
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

    /// Get a device by its address and address type
    /// Address alone is not unique - must be combined with address_type for proper lookup
    pub async fn get_device_by_address(
        &self,
        address: u32,
        address_type: AddressType,
    ) -> Result<Option<Device>> {
        let mut conn = self.get_connection()?;
        let device_model = devices::table
            .filter(devices::address.eq(address as i32))
            .filter(devices::address_type.eq(address_type))
            .first::<DeviceModel>(&mut conn)
            .optional()?;

        Ok(device_model.map(|model| model.into()))
    }

    /// Get a device model (with UUID) by address and address type
    pub async fn get_device_model_by_address(
        &self,
        address: u32,
        address_type: AddressType,
    ) -> Result<Option<DeviceModel>> {
        let mut conn = self.get_connection()?;
        let device_model = devices::table
            .filter(devices::address.eq(address as i32))
            .filter(devices::address_type.eq(address_type))
            .first::<DeviceModel>(&mut conn)
            .optional()?;
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

    /// Get all devices (aircraft) assigned to a specific club
    pub async fn get_devices_by_club_id(&self, club_id: Uuid) -> Result<Vec<Device>> {
        let mut conn = self.get_connection()?;

        // This query requires joining with aircraft_registrations table
        // We'll use raw SQL for now since it involves a join with another table
        let sql = r#"
            SELECT d.address, d.address_type, d.aircraft_model, d.registration,
                   d.competition_number, d.tracked, d.identified, d.created_at, d.updated_at, d.id
            FROM devices d
            INNER JOIN aircraft_registrations ar ON d.registration = ar.registration_number
            WHERE ar.club_id = $1
            ORDER BY d.registration
        "#;

        let device_models: Vec<DeviceModel> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(club_id)
            .load(&mut conn)?;

        Ok(device_models
            .into_iter()
            .map(|model| model.into())
            .collect())
    }

    /// Search devices by address (returns all devices with this address across all address types)
    pub async fn search_by_address(&self, address: u32) -> Result<Vec<Device>> {
        let mut conn = self.get_connection()?;
        let device_models = devices::table
            .filter(devices::address.eq(address as i32))
            .load::<DeviceModel>(&mut conn)?;

        Ok(device_models
            .into_iter()
            .map(|model| model.into())
            .collect())
    }

    /// Search devices by address and address type combination
    /// Returns a single device since (address, address_type) is a unique key
    pub async fn search_by_address_and_type(
        &self,
        address: u32,
        address_type: AddressType,
    ) -> Result<Option<Device>> {
        let mut conn = self.get_connection()?;
        let device_model = devices::table
            .filter(devices::address.eq(address as i32))
            .filter(devices::address_type.eq(address_type))
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
