use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use tracing::{info, warn};
use uuid::Uuid;

use crate::aircraft_registrations::{Aircraft, AircraftRegistrationModel, NewAircraftRegistration};
use crate::schema::aircraft_registrations;

pub type DieselPgPool = Pool<ConnectionManager<PgConnection>>;
pub type DieselPgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

pub struct AircraftRegistrationsRepository {
    pool: DieselPgPool,
}

impl AircraftRegistrationsRepository {
    pub fn new(pool: DieselPgPool) -> Self {
        Self { pool }
    }
    
    fn get_connection(&self) -> Result<DieselPgPooledConnection> {
        self.pool.get().map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))
    }

    /// Upsert aircraft registrations into the database
    /// This will insert new aircraft registrations or update existing ones based on the primary key (registration_number)
    pub async fn upsert_aircraft_registrations<I>(&self, aircraft: I) -> Result<usize>
    where
        I: IntoIterator<Item = Aircraft>,
    {
        let mut conn = self.get_connection()?;
        let mut upserted_count = 0;

        // Convert aircraft to NewAircraftRegistration structs for insertion
        let new_aircraft: Vec<NewAircraftRegistration> = aircraft.into_iter().map(|a| a.into()).collect();

        for new_aircraft_reg in new_aircraft {
            let result = diesel::insert_into(aircraft_registrations::table)
                .values(&new_aircraft_reg)
                .on_conflict(aircraft_registrations::registration_number)
                .do_update()
                .set((
                    aircraft_registrations::serial_number.eq(excluded(aircraft_registrations::serial_number)),
                    aircraft_registrations::mfr_mdl_code.eq(excluded(aircraft_registrations::mfr_mdl_code)),
                    aircraft_registrations::eng_mfr_mdl_code.eq(excluded(aircraft_registrations::eng_mfr_mdl_code)),
                    aircraft_registrations::year_mfr.eq(excluded(aircraft_registrations::year_mfr)),
                    aircraft_registrations::type_registration_code.eq(excluded(aircraft_registrations::type_registration_code)),
                    aircraft_registrations::registrant_name.eq(excluded(aircraft_registrations::registrant_name)),
                    aircraft_registrations::location_id.eq(excluded(aircraft_registrations::location_id)),
                    aircraft_registrations::last_action_date.eq(excluded(aircraft_registrations::last_action_date)),
                    aircraft_registrations::certificate_issue_date.eq(excluded(aircraft_registrations::certificate_issue_date)),
                    aircraft_registrations::airworthiness_class.eq(excluded(aircraft_registrations::airworthiness_class)),
                    aircraft_registrations::approved_operations_raw.eq(excluded(aircraft_registrations::approved_operations_raw)),
                    aircraft_registrations::type_aircraft_code.eq(excluded(aircraft_registrations::type_aircraft_code)),
                    aircraft_registrations::type_engine_code.eq(excluded(aircraft_registrations::type_engine_code)),
                    aircraft_registrations::status_code.eq(excluded(aircraft_registrations::status_code)),
                    aircraft_registrations::transponder_code.eq(excluded(aircraft_registrations::transponder_code)),
                    aircraft_registrations::fractional_owner.eq(excluded(aircraft_registrations::fractional_owner)),
                    aircraft_registrations::airworthiness_date.eq(excluded(aircraft_registrations::airworthiness_date)),
                    aircraft_registrations::expiration_date.eq(excluded(aircraft_registrations::expiration_date)),
                    aircraft_registrations::unique_id.eq(excluded(aircraft_registrations::unique_id)),
                    aircraft_registrations::kit_mfr_name.eq(excluded(aircraft_registrations::kit_mfr_name)),
                    aircraft_registrations::kit_model_name.eq(excluded(aircraft_registrations::kit_model_name)),
                    aircraft_registrations::device_id.eq(excluded(aircraft_registrations::device_id)),
                ))
                .execute(&mut conn);

            match result {
                Ok(_) => {
                    upserted_count += 1;
                }
                Err(e) => {
                    warn!("Failed to upsert aircraft registration {}: {}", new_aircraft_reg.registration_number, e);
                    // Continue with other aircraft rather than failing the entire batch
                }
            }
        }

        info!("Successfully upserted {} aircraft registrations", upserted_count);
        Ok(upserted_count)
    }

    /// Get the total count of aircraft registrations in the database
    pub async fn get_aircraft_registration_count(&self) -> Result<i64> {
        let mut conn = self.get_connection()?;
        let count = aircraft_registrations::table
            .count()
            .get_result::<i64>(&mut conn)?;
        Ok(count)
    }

    /// Get an aircraft registration by its registration number (N-number)
    pub async fn get_aircraft_registration_by_n_number(
        &self,
        registration_number: &str,
    ) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let aircraft_model = aircraft_registrations::table
            .filter(aircraft_registrations::registration_number.eq(registration_number))
            .first::<AircraftRegistrationModel>(&mut conn)
            .optional()?;

        Ok(aircraft_model.map(|model| model.into()))
    }

    /// Search aircraft registrations by registrant name
    pub async fn search_by_registrant_name(&self, registrant_name: &str) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;
        let search_pattern = format!("%{}%", registrant_name);
        let aircraft_models = aircraft_registrations::table
            .filter(aircraft_registrations::registrant_name.ilike(&search_pattern))
            .load::<AircraftRegistrationModel>(&mut conn)?;

        Ok(aircraft_models.into_iter().map(|model| model.into()).collect())
    }

    /// Search aircraft registrations by transponder code
    pub async fn search_by_transponder_code(&self, transponder_code: u32) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;
        let transponder_code_i64 = transponder_code as i64;
        let aircraft_models = aircraft_registrations::table
            .filter(aircraft_registrations::transponder_code.eq(transponder_code_i64))
            .load::<AircraftRegistrationModel>(&mut conn)?;

        Ok(aircraft_models.into_iter().map(|model| model.into()).collect())
    }

    /// Search aircraft registrations by state
    pub async fn search_by_state(&self, _state: &str) -> Result<Vec<Aircraft>> {
        // Note: State is no longer stored directly in aircraft_registrations table.
        // It's now in the locations table via location_id.
        // For now, return empty result until locations integration is implemented.
        warn!("search_by_state not fully implemented - state data is now in locations table");
        Ok(Vec::new())
    }

    /// Get aircraft registrations by club ID
    pub async fn get_by_club_id(&self, club_id: Uuid) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;
        let aircraft_models = aircraft_registrations::table
            .filter(aircraft_registrations::club_id.eq(club_id))
            .load::<AircraftRegistrationModel>(&mut conn)?;

        Ok(aircraft_models.into_iter().map(|model| model.into()).collect())
    }
}