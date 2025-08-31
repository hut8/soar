use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::faa::models::{AircraftModel, AircraftType, EngineType, AircraftCategory, BuilderCertification, WeightClass};

pub struct AircraftModelRepository {
    pool: PgPool,
}

impl AircraftModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert aircraft models into the database
    /// This will insert new aircraft models or update existing ones based on the composite primary key
    /// (manufacturer_code, model_code, series_code)
    pub async fn upsert_aircraft_models<I>(&self, aircraft_models: I) -> Result<usize>
    where
        I: IntoIterator<Item = AircraftModel>,
    {
        let mut transaction = self.pool.begin().await?;
        let mut upserted_count = 0;

        for aircraft_model in aircraft_models {
            // Convert enum types to strings for database storage
            let aircraft_type_str = aircraft_model.aircraft_type.as_ref().map(|t| t.to_string());
            let engine_type_str = aircraft_model.engine_type.as_ref().map(|t| t.to_string());
            let aircraft_category_str = aircraft_model.aircraft_category.as_ref().map(|t| t.to_string());
            let builder_certification_str = aircraft_model.builder_certification.as_ref().map(|t| t.to_string());
            let weight_class_str = aircraft_model.weight_class.as_ref().map(|t| t.to_string());

            // Convert u16 to i16 for database storage (SMALLINT)
            let number_of_engines = aircraft_model.number_of_engines.map(|n| n as i16);
            let number_of_seats = aircraft_model.number_of_seats.map(|n| n as i16);
            let cruising_speed = aircraft_model.cruising_speed.map(|n| n as i16);

            // Use ON CONFLICT to handle upserts
            let result = sqlx::query!(
                r#"
                INSERT INTO aircraft_model (
                    manufacturer_code, model_code, series_code, manufacturer_name, model_name,
                    aircraft_type, engine_type, aircraft_category, builder_certification,
                    number_of_engines, number_of_seats, weight_class, cruising_speed,
                    type_certificate_data_sheet, type_certificate_data_holder
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                ON CONFLICT (manufacturer_code, model_code, series_code)
                DO UPDATE SET
                    manufacturer_name = EXCLUDED.manufacturer_name,
                    model_name = EXCLUDED.model_name,
                    aircraft_type = EXCLUDED.aircraft_type,
                    engine_type = EXCLUDED.engine_type,
                    aircraft_category = EXCLUDED.aircraft_category,
                    builder_certification = EXCLUDED.builder_certification,
                    number_of_engines = EXCLUDED.number_of_engines,
                    number_of_seats = EXCLUDED.number_of_seats,
                    weight_class = EXCLUDED.weight_class,
                    cruising_speed = EXCLUDED.cruising_speed,
                    type_certificate_data_sheet = EXCLUDED.type_certificate_data_sheet,
                    type_certificate_data_holder = EXCLUDED.type_certificate_data_holder,
                    updated_at = NOW()
                "#,
                aircraft_model.manufacturer_code,
                aircraft_model.model_code,
                aircraft_model.series_code,
                aircraft_model.manufacturer_name,
                aircraft_model.model_name,
                aircraft_type_str,
                engine_type_str,
                aircraft_category_str,
                builder_certification_str,
                number_of_engines,
                number_of_seats,
                weight_class_str,
                cruising_speed,
                aircraft_model.type_certificate_data_sheet,
                aircraft_model.type_certificate_data_holder
            )
            .execute(&mut *transaction)
            .await;

            match result {
                Ok(_) => {
                    upserted_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to upsert aircraft model {}-{}-{}: {}",
                        aircraft_model.manufacturer_code,
                        aircraft_model.model_code,
                        aircraft_model.series_code,
                        e
                    );
                    // Continue with other aircraft models rather than failing the entire batch
                }
            }
        }

        transaction.commit().await?;
        info!("Successfully upserted {} aircraft models", upserted_count);

        Ok(upserted_count)
    }

    /// Get the total count of aircraft models in the database
    pub async fn get_aircraft_model_count(&self) -> Result<i64> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM aircraft_model")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Get an aircraft model by its composite primary key
    pub async fn get_aircraft_model_by_key(
        &self,
        manufacturer_code: &str,
        model_code: &str,
        series_code: &str,
    ) -> Result<Option<AircraftModel>> {
        let result = sqlx::query!(
            r#"
            SELECT manufacturer_code, model_code, series_code, manufacturer_name, model_name,
                   aircraft_type, engine_type, aircraft_category, builder_certification,
                   number_of_engines, number_of_seats, weight_class, cruising_speed,
                   type_certificate_data_sheet, type_certificate_data_holder
            FROM aircraft_model
            WHERE manufacturer_code = $1 AND model_code = $2 AND series_code = $3
            "#,
            manufacturer_code,
            model_code,
            series_code
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            // Convert string types back to enums
            let aircraft_type = row.aircraft_type
                .as_ref()
                .map(|s| s.parse::<AircraftType>())
                .transpose()?;

            let engine_type = row.engine_type
                .as_ref()
                .map(|s| s.parse::<EngineType>())
                .transpose()?;

            let aircraft_category = row.aircraft_category
                .as_ref()
                .map(|s| s.parse::<AircraftCategory>())
                .transpose()?;

            let builder_certification = row.builder_certification
                .as_ref()
                .map(|s| s.parse::<BuilderCertification>())
                .transpose()?;

            let weight_class = row.weight_class
                .as_ref()
                .map(|s| s.parse::<WeightClass>())
                .transpose()?;

            // Convert i16 back to u16
            let number_of_engines = row.number_of_engines.map(|n| n as u16);
            let number_of_seats = row.number_of_seats.map(|n| n as u16);
            let cruising_speed = row.cruising_speed.map(|n| n as u16);

            Ok(Some(AircraftModel {
                manufacturer_code: row.manufacturer_code,
                model_code: row.model_code,
                series_code: row.series_code,
                manufacturer_name: row.manufacturer_name,
                model_name: row.model_name,
                aircraft_type,
                engine_type,
                aircraft_category,
                builder_certification,
                number_of_engines,
                number_of_seats,
                weight_class,
                cruising_speed,
                type_certificate_data_sheet: row.type_certificate_data_sheet,
                type_certificate_data_holder: row.type_certificate_data_holder,
            }))
        } else {
            Ok(None)
        }
    }

    /// Search aircraft models by manufacturer name (case-insensitive partial match)
    pub async fn search_by_manufacturer(&self, manufacturer_name: &str) -> Result<Vec<AircraftModel>> {
        let results = sqlx::query!(
            r#"
            SELECT manufacturer_code, model_code, series_code, manufacturer_name, model_name,
                   aircraft_type, engine_type, aircraft_category, builder_certification,
                   number_of_engines, number_of_seats, weight_class, cruising_speed,
                   type_certificate_data_sheet, type_certificate_data_holder
            FROM aircraft_model
            WHERE manufacturer_name ILIKE $1
            ORDER BY manufacturer_name, model_name
            "#,
            format!("%{}%", manufacturer_name)
        )
        .fetch_all(&self.pool)
        .await?;

        let mut aircraft_models = Vec::new();
        for row in results {
            // Convert string types back to enums
            let aircraft_type = row.aircraft_type
                .as_ref()
                .map(|s| s.parse::<AircraftType>())
                .transpose()?;

            let engine_type = row.engine_type
                .as_ref()
                .map(|s| s.parse::<EngineType>())
                .transpose()?;

            let aircraft_category = row.aircraft_category
                .as_ref()
                .map(|s| s.parse::<AircraftCategory>())
                .transpose()?;

            let builder_certification = row.builder_certification
                .as_ref()
                .map(|s| s.parse::<BuilderCertification>())
                .transpose()?;

            let weight_class = row.weight_class
                .as_ref()
                .map(|s| s.parse::<WeightClass>())
                .transpose()?;

            // Convert i16 back to u16
            let number_of_engines = row.number_of_engines.map(|n| n as u16);
            let number_of_seats = row.number_of_seats.map(|n| n as u16);
            let cruising_speed = row.cruising_speed.map(|n| n as u16);

            aircraft_models.push(AircraftModel {
                manufacturer_code: row.manufacturer_code,
                model_code: row.model_code,
                series_code: row.series_code,
                manufacturer_name: row.manufacturer_name,
                model_name: row.model_name,
                aircraft_type,
                engine_type,
                aircraft_category,
                builder_certification,
                number_of_engines,
                number_of_seats,
                weight_class,
                cruising_speed,
                type_certificate_data_sheet: row.type_certificate_data_sheet,
                type_certificate_data_holder: row.type_certificate_data_holder,
            });
        }

        Ok(aircraft_models)
    }

    /// Search aircraft models by model name (case-insensitive partial match)
    pub async fn search_by_model(&self, model_name: &str) -> Result<Vec<AircraftModel>> {
        let results = sqlx::query!(
            r#"
            SELECT manufacturer_code, model_code, series_code, manufacturer_name, model_name,
                   aircraft_type, engine_type, aircraft_category, builder_certification,
                   number_of_engines, number_of_seats, weight_class, cruising_speed,
                   type_certificate_data_sheet, type_certificate_data_holder
            FROM aircraft_model
            WHERE model_name ILIKE $1
            ORDER BY manufacturer_name, model_name
            "#,
            format!("%{}%", model_name)
        )
        .fetch_all(&self.pool)
        .await?;

        let mut aircraft_models = Vec::new();
        for row in results {
            // Convert string types back to enums (reusing the same conversion logic)
            let aircraft_type = row.aircraft_type
                .as_ref()
                .map(|s| s.parse::<AircraftType>())
                .transpose()?;

            let engine_type = row.engine_type
                .as_ref()
                .map(|s| s.parse::<EngineType>())
                .transpose()?;

            let aircraft_category = row.aircraft_category
                .as_ref()
                .map(|s| s.parse::<AircraftCategory>())
                .transpose()?;

            let builder_certification = row.builder_certification
                .as_ref()
                .map(|s| s.parse::<BuilderCertification>())
                .transpose()?;

            let weight_class = row.weight_class
                .as_ref()
                .map(|s| s.parse::<WeightClass>())
                .transpose()?;

            let number_of_engines = row.number_of_engines.map(|n| n as u16);
            let number_of_seats = row.number_of_seats.map(|n| n as u16);
            let cruising_speed = row.cruising_speed.map(|n| n as u16);

            aircraft_models.push(AircraftModel {
                manufacturer_code: row.manufacturer_code,
                model_code: row.model_code,
                series_code: row.series_code,
                manufacturer_name: row.manufacturer_name,
                model_name: row.model_name,
                aircraft_type,
                engine_type,
                aircraft_category,
                builder_certification,
                number_of_engines,
                number_of_seats,
                weight_class,
                cruising_speed,
                type_certificate_data_sheet: row.type_certificate_data_sheet,
                type_certificate_data_holder: row.type_certificate_data_holder,
            });
        }

        Ok(aircraft_models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faa::models::{AircraftType, EngineType, AircraftCategory, BuilderCertification, WeightClass};

    // Note: These tests would require a test database setup
    // For now, they're just structural examples

    fn create_test_aircraft_model() -> AircraftModel {
        AircraftModel {
            manufacturer_code: "ABC".to_string(),
            model_code: "12".to_string(),
            series_code: "34".to_string(),
            manufacturer_name: "Test Manufacturer".to_string(),
            model_name: "Test Model".to_string(),
            aircraft_type: Some(AircraftType::FixedWingSingleEngine),
            engine_type: Some(EngineType::Reciprocating),
            aircraft_category: Some(AircraftCategory::Land),
            builder_certification: Some(BuilderCertification::TypeCertificated),
            number_of_engines: Some(1),
            number_of_seats: Some(4),
            weight_class: Some(WeightClass::UpTo12499),
            cruising_speed: Some(120),
            type_certificate_data_sheet: Some("A23CE".to_string()),
            type_certificate_data_holder: Some("Test Certificate Holder".to_string()),
        }
    }

    #[test]
    fn test_aircraft_model_creation() {
        let aircraft_model = create_test_aircraft_model();
        assert_eq!(aircraft_model.manufacturer_code, "ABC");
        assert_eq!(aircraft_model.model_code, "12");
        assert_eq!(aircraft_model.series_code, "34");
        assert_eq!(aircraft_model.manufacturer_name, "Test Manufacturer");
        assert_eq!(aircraft_model.model_name, "Test Model");
        assert_eq!(aircraft_model.aircraft_type, Some(AircraftType::FixedWingSingleEngine));
        assert_eq!(aircraft_model.engine_type, Some(EngineType::Reciprocating));
    }
}
