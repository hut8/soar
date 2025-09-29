use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::faa::aircraft_models::{
    AircraftCategory, AircraftModel, AircraftType, BuilderCertification, EngineType, WeightClass,
};
use crate::schema::aircraft_models;

pub type DieselPgPool = Pool<ConnectionManager<PgConnection>>;
pub type DieselPgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

/// Diesel model for the aircraft_model table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = aircraft_models)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AircraftModelRecord {
    pub manufacturer_code: String,
    pub model_code: String,
    pub series_code: String,
    pub manufacturer_name: String,
    pub model_name: String,
    pub aircraft_type: Option<String>,
    pub engine_type: Option<String>,
    pub aircraft_category: Option<String>,
    pub builder_certification: Option<String>,
    pub number_of_engines: Option<i16>,
    pub number_of_seats: Option<i16>,
    pub weight_class: Option<String>,
    pub cruising_speed: Option<i16>,
    pub type_certificate_data_sheet: Option<String>,
    pub type_certificate_data_holder: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new aircraft models
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = aircraft_models)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAircraftModelRecord {
    pub manufacturer_code: String,
    pub model_code: String,
    pub series_code: String,
    pub manufacturer_name: String,
    pub model_name: String,
    pub aircraft_type: Option<String>,
    pub engine_type: Option<String>,
    pub aircraft_category: Option<String>,
    pub builder_certification: Option<String>,
    pub number_of_engines: Option<i16>,
    pub number_of_seats: Option<i16>,
    pub weight_class: Option<String>,
    pub cruising_speed: Option<i16>,
    pub type_certificate_data_sheet: Option<String>,
    pub type_certificate_data_holder: Option<String>,
}

/// Conversion from AircraftModel (API model) to AircraftModelRecord (database model)
impl From<AircraftModel> for AircraftModelRecord {
    fn from(model: AircraftModel) -> Self {
        Self {
            manufacturer_code: model.manufacturer_code,
            model_code: model.model_code,
            series_code: model.series_code,
            manufacturer_name: model.manufacturer_name,
            model_name: model.model_name,
            aircraft_type: model.aircraft_type.map(|t| t.to_string()),
            engine_type: model.engine_type.map(|t| t.to_string()),
            aircraft_category: model.aircraft_category.map(|t| t.to_string()),
            builder_certification: model.builder_certification.map(|t| t.to_string()),
            number_of_engines: model.number_of_engines.map(|n| n as i16),
            number_of_seats: model.number_of_seats.map(|n| n as i16),
            weight_class: model.weight_class.map(|t| t.to_string()),
            cruising_speed: model.cruising_speed.map(|n| n as i16),
            type_certificate_data_sheet: model.type_certificate_data_sheet,
            type_certificate_data_holder: model.type_certificate_data_holder,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Conversion from AircraftModel (API model) to NewAircraftModelRecord (insert model)
impl From<AircraftModel> for NewAircraftModelRecord {
    fn from(model: AircraftModel) -> Self {
        Self {
            manufacturer_code: model.manufacturer_code,
            model_code: model.model_code,
            series_code: model.series_code,
            manufacturer_name: model.manufacturer_name,
            model_name: model.model_name,
            aircraft_type: model.aircraft_type.map(|t| t.to_string()),
            engine_type: model.engine_type.map(|t| t.to_string()),
            aircraft_category: model.aircraft_category.map(|t| t.to_string()),
            builder_certification: model.builder_certification.map(|t| t.to_string()),
            number_of_engines: model.number_of_engines.map(|n| n as i16),
            number_of_seats: model.number_of_seats.map(|n| n as i16),
            weight_class: model.weight_class.map(|t| t.to_string()),
            cruising_speed: model.cruising_speed.map(|n| n as i16),
            type_certificate_data_sheet: model.type_certificate_data_sheet,
            type_certificate_data_holder: model.type_certificate_data_holder,
        }
    }
}

/// Conversion from AircraftModelRecord (database model) to AircraftModel (API model)
impl TryFrom<AircraftModelRecord> for AircraftModel {
    type Error = anyhow::Error;

    fn try_from(record: AircraftModelRecord) -> Result<Self> {
        // Convert string types back to enums
        let aircraft_type = record
            .aircraft_type
            .as_ref()
            .map(|s| s.parse::<AircraftType>())
            .transpose()?;

        let engine_type = record
            .engine_type
            .as_ref()
            .map(|s| s.parse::<EngineType>())
            .transpose()?;

        let aircraft_category = record
            .aircraft_category
            .as_ref()
            .map(|s| s.parse::<AircraftCategory>())
            .transpose()?;

        let builder_certification = record
            .builder_certification
            .as_ref()
            .map(|s| s.parse::<BuilderCertification>())
            .transpose()?;

        let weight_class = record
            .weight_class
            .as_ref()
            .map(|s| s.parse::<WeightClass>())
            .transpose()?;

        // Convert i16 back to u16
        let number_of_engines = record.number_of_engines.map(|n| n as u16);
        let number_of_seats = record.number_of_seats.map(|n| n as u16);
        let cruising_speed = record.cruising_speed.map(|n| n as u16);

        Ok(AircraftModel {
            manufacturer_code: record.manufacturer_code,
            model_code: record.model_code,
            series_code: record.series_code,
            manufacturer_name: record.manufacturer_name,
            model_name: record.model_name,
            aircraft_type,
            engine_type,
            aircraft_category,
            builder_certification,
            number_of_engines,
            number_of_seats,
            weight_class,
            cruising_speed,
            type_certificate_data_sheet: record.type_certificate_data_sheet,
            type_certificate_data_holder: record.type_certificate_data_holder,
        })
    }
}

pub struct AircraftModelRepository {
    pool: DieselPgPool,
}

impl AircraftModelRepository {
    pub fn new(pool: DieselPgPool) -> Self {
        Self { pool }
    }

    /// Upsert aircraft models into the database
    /// This will insert new aircraft models or update existing ones based on the composite primary key
    /// (manufacturer_code, model_code, series_code)
    pub async fn upsert_aircraft_models<I>(&self, aircraft_models: I) -> Result<usize>
    where
        I: IntoIterator<Item = AircraftModel>,
    {
        use chrono::Utc;

        let pool = self.pool.clone();
        let models: Vec<AircraftModel> = aircraft_models.into_iter().collect();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let mut upserted_count = 0;

            // Use a transaction for the batch operation
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                for model in models {
                    let new_record = NewAircraftModelRecord::from(model.clone());

                    // Use Diesel's ON CONFLICT functionality
                    let result = diesel::insert_into(aircraft_models::table)
                        .values(&new_record)
                        .on_conflict((
                            aircraft_models::manufacturer_code,
                            aircraft_models::model_code,
                            aircraft_models::series_code,
                        ))
                        .do_update()
                        .set((
                            aircraft_models::manufacturer_name
                                .eq(excluded(aircraft_models::manufacturer_name)),
                            aircraft_models::model_name.eq(excluded(aircraft_models::model_name)),
                            aircraft_models::aircraft_type
                                .eq(excluded(aircraft_models::aircraft_type)),
                            aircraft_models::engine_type.eq(excluded(aircraft_models::engine_type)),
                            aircraft_models::aircraft_category
                                .eq(excluded(aircraft_models::aircraft_category)),
                            aircraft_models::builder_certification
                                .eq(excluded(aircraft_models::builder_certification)),
                            aircraft_models::number_of_engines
                                .eq(excluded(aircraft_models::number_of_engines)),
                            aircraft_models::number_of_seats
                                .eq(excluded(aircraft_models::number_of_seats)),
                            aircraft_models::weight_class
                                .eq(excluded(aircraft_models::weight_class)),
                            aircraft_models::cruising_speed
                                .eq(excluded(aircraft_models::cruising_speed)),
                            aircraft_models::type_certificate_data_sheet
                                .eq(excluded(aircraft_models::type_certificate_data_sheet)),
                            aircraft_models::type_certificate_data_holder
                                .eq(excluded(aircraft_models::type_certificate_data_holder)),
                            aircraft_models::updated_at.eq(Utc::now()),
                        ))
                        .execute(conn);

                    match result {
                        Ok(_) => {
                            upserted_count += 1;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to upsert aircraft model {}-{}-{}: {}",
                                model.manufacturer_code, model.model_code, model.series_code, e
                            );
                            // Continue with other aircraft models rather than failing the entire batch
                        }
                    }
                }

                Ok(())
            })?;

            info!("Successfully upserted {} aircraft models", upserted_count);
            Ok::<usize, anyhow::Error>(upserted_count)
        })
        .await?
    }

    /// Get the total count of aircraft models in the database
    pub async fn get_aircraft_model_count(&self) -> Result<i64> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count = aircraft_models::table
                .count()
                .get_result::<i64>(&mut conn)?;
            Ok::<i64, anyhow::Error>(count)
        })
        .await?
    }

    /// Get an aircraft model by its composite primary key
    pub async fn get_aircraft_model_by_key(
        &self,
        manufacturer_code_param: &str,
        model_code_param: &str,
        series_code_param: &str,
    ) -> Result<Option<AircraftModel>> {
        let pool = self.pool.clone();
        let manufacturer_code_param = manufacturer_code_param.to_string();
        let model_code_param = model_code_param.to_string();
        let series_code_param = series_code_param.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let result = aircraft_models::table
                .filter(aircraft_models::manufacturer_code.eq(&manufacturer_code_param))
                .filter(aircraft_models::model_code.eq(&model_code_param))
                .filter(aircraft_models::series_code.eq(&series_code_param))
                .select(AircraftModelRecord::as_select())
                .first::<AircraftModelRecord>(&mut conn)
                .optional()?;

            match result {
                Some(record) => {
                    let model = AircraftModel::try_from(record)?;
                    Ok(Some(model))
                }
                None => Ok(None),
            }
        })
        .await?
    }

    /// Search aircraft models by manufacturer name (case-insensitive partial match)
    pub async fn search_by_manufacturer(
        &self,
        manufacturer_name_param: &str,
    ) -> Result<Vec<AircraftModel>> {
        let pool = self.pool.clone();
        let search_pattern = format!("%{}%", manufacturer_name_param);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let results = aircraft_models::table
                .filter(aircraft_models::manufacturer_name.ilike(&search_pattern))
                .order((
                    aircraft_models::manufacturer_name.asc(),
                    aircraft_models::model_name.asc(),
                ))
                .load::<AircraftModelRecord>(&mut conn)?;

            let mut aircraft_models = Vec::new();
            for record in results {
                let model = AircraftModel::try_from(record)?;
                aircraft_models.push(model);
            }

            Ok::<Vec<AircraftModel>, anyhow::Error>(aircraft_models)
        })
        .await?
    }

    /// Search aircraft models by model name (case-insensitive partial match)
    pub async fn search_by_model(&self, model_name_param: &str) -> Result<Vec<AircraftModel>> {
        let pool = self.pool.clone();
        let search_pattern = format!("%{}%", model_name_param);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let results = aircraft_models::table
                .filter(aircraft_models::model_name.ilike(&search_pattern))
                .order((
                    aircraft_models::manufacturer_name.asc(),
                    aircraft_models::model_name.asc(),
                ))
                .load::<AircraftModelRecord>(&mut conn)?;

            let mut aircraft_models = Vec::new();
            for record in results {
                let model = AircraftModel::try_from(record)?;
                aircraft_models.push(model);
            }

            Ok::<Vec<AircraftModel>, anyhow::Error>(aircraft_models)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faa::aircraft_models::{
        AircraftCategory, AircraftType, BuilderCertification, EngineType, WeightClass,
    };

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
        assert_eq!(
            aircraft_model.aircraft_type,
            Some(AircraftType::FixedWingSingleEngine)
        );
        assert_eq!(aircraft_model.engine_type, Some(EngineType::Reciprocating));
    }
}
