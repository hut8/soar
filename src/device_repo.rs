use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::ddb::{Device, DeviceType};

pub struct DeviceRepository {
    pool: PgPool,
}

impl DeviceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert devices into the database
    /// This will insert new devices or update existing ones based on device_id
    pub async fn upsert_devices<I>(&self, devices: I) -> Result<usize>
    where
        I: IntoIterator<Item = Device>,
    {
        let mut transaction = self.pool.begin().await?;
        let mut upserted_count = 0;

        for device in devices {
            // Convert DeviceType to string for database storage
            let device_type_str = match device.device_type {
                DeviceType::Flarm => "F",
                DeviceType::Ogn => "O",
                DeviceType::Icao => "I",
                DeviceType::Unknown => "",
            };

            // Convert tracked and identified strings to booleans
            let tracked = device.tracked.to_uppercase() == "Y";
            let identified = device.identified.to_uppercase() == "Y";

            // Use ON CONFLICT to handle upserts
            let result = sqlx::query!(
                r#"
                INSERT INTO devices (device_id, device_type, aircraft_model, registration, cn, tracked, identified)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (device_id)
                DO UPDATE SET
                    device_type = EXCLUDED.device_type,
                    aircraft_model = EXCLUDED.aircraft_model,
                    registration = EXCLUDED.registration,
                    cn = EXCLUDED.cn,
                    tracked = EXCLUDED.tracked,
                    identified = EXCLUDED.identified,
                    updated_at = NOW()
                "#,
                device.device_id,
                device_type_str,
                device.aircraft_model,
                device.registration,
                device.competition_number,
                tracked,
                identified
            )
            .execute(&mut *transaction)
            .await;

            match result {
                Ok(_) => {
                    upserted_count += 1;
                }
                Err(e) => {
                    warn!("Failed to upsert device {}: {}", device.device_id, e);
                    // Continue with other devices rather than failing the entire batch
                }
            }
        }

        transaction.commit().await?;
        info!("Successfully upserted {} devices", upserted_count);

        Ok(upserted_count)
    }

    /// Get the total count of devices in the database
    pub async fn get_device_count(&self) -> Result<i64> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM devices")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Get a device by its device_id
    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Option<Device>> {
        let result = sqlx::query!(
            "SELECT device_id, device_type, aircraft_model, registration, cn, tracked, identified FROM devices WHERE device_id = $1",
            device_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let device_type = match row.device_type.as_str() {
                "F" => DeviceType::Flarm,
                "O" => DeviceType::Ogn,
                "I" => DeviceType::Icao,
                _ => DeviceType::Unknown,
            };

            let tracked = if row.tracked { "Y" } else { "N" };
            let identified = if row.identified { "Y" } else { "N" };

            Ok(Some(Device {
                device_id: row.device_id,
                device_type,
                aircraft_model: row.aircraft_model,
                registration: row.registration,
                competition_number: row.cn,
                tracked: tracked.to_string(),
                identified: identified.to_string(),
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ddb::DeviceType;

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_device() -> Device {
        Device {
            device_id: "123456".to_string(),
            device_type: DeviceType::Flarm,
            aircraft_model: "Test Aircraft".to_string(),
            registration: "N123AB".to_string(),
            competition_number: "42".to_string(),
            tracked: "Y".to_string(),
            identified: "Y".to_string(),
        }
    }

    #[test]
    fn test_device_creation() {
        let device = create_test_device();
        assert_eq!(device.device_id, "123456");
        assert_eq!(device.device_type, DeviceType::Flarm);
    }
}
