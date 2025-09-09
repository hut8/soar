use anyhow::Result;
use sqlx::PgPool;
use std::str::FromStr;
use tracing::{info, warn};

use crate::devices::{Device, DeviceType};

#[derive(Clone)]
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
            // Convert enum to string for database storage
            let device_type_str = device.device_type.to_string();

            // Use ON CONFLICT to handle upserts
            let result = sqlx::query!(
                r#"
                INSERT INTO devices (
                    device_type, device_id, aircraft_model, registration,
                    competition_number, tracked, identified
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (device_id)
                DO UPDATE SET
                    device_type = EXCLUDED.device_type,
                    aircraft_model = EXCLUDED.aircraft_model,
                    registration = EXCLUDED.registration,
                    competition_number = EXCLUDED.competition_number,
                    tracked = EXCLUDED.tracked,
                    identified = EXCLUDED.identified,
                    updated_at = CURRENT_TIMESTAMP
                "#,
                device_type_str,
                device.device_id as i32,
                device.aircraft_model,
                device.registration,
                device.competition_number,
                device.tracked,
                device.identified
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
    pub async fn get_device_by_id(&self, device_id: u32) -> Result<Option<Device>> {
        let row = sqlx::query!(
            r#"
            SELECT device_type, device_id, aircraft_model, registration,
                   competition_number, tracked, identified
            FROM devices
            WHERE device_id = $1
            "#,
            device_id as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let device_type = DeviceType::from_str(&row.device_type).unwrap_or(DeviceType::Unknown);

            Ok(Some(Device {
                device_type,
                device_id: row.device_id as u32,
                aircraft_model: row.aircraft_model,
                registration: row.registration,
                competition_number: row.competition_number,
                tracked: row.tracked,
                identified: row.identified,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all devices (aircraft) assigned to a specific club
    pub async fn get_devices_by_club_id(&self, club_id: sqlx::types::Uuid) -> Result<Vec<Device>> {
        let rows = sqlx::query!(
            r#"
            SELECT d.device_type, d.device_id, d.aircraft_model, d.registration,
                   d.competition_number, d.tracked, d.identified
            FROM devices d
            INNER JOIN aircraft_registrations ar ON d.registration = ar.registration_number
            WHERE ar.club_id = $1
            ORDER BY d.registration
            "#,
            club_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut devices = Vec::new();
        for row in rows {
            let device_type = DeviceType::from_str(&row.device_type).unwrap_or(DeviceType::Unknown);

            devices.push(Device {
                device_type,
                device_id: row.device_id as u32,
                aircraft_model: row.aircraft_model,
                registration: row.registration,
                competition_number: row.competition_number,
                tracked: row.tracked,
                identified: row.identified,
            });
        }

        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::DeviceType;

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_device() -> Device {
        Device {
            device_id: 123456,
            device_type: DeviceType::Flarm,
            aircraft_model: "Test Aircraft".to_string(),
            registration: "N123AB".to_string(),
            competition_number: "42".to_string(),
            tracked: true,
            identified: true,
        }
    }

    #[test]
    fn test_device_creation() {
        let device = create_test_device();
        assert_eq!(device.device_id, 123456);
        assert_eq!(device.device_type, DeviceType::Flarm);
    }
}
