use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::aircraft_registrations::{Aircraft, AircraftRegistrationModel, NewAircraftRegistration};
use crate::clubs_repo::ClubsRepository;
use crate::locations_repo::LocationsRepository;
use crate::schema::{aircraft_approved_operations, aircraft_other_names, aircraft_registrations};

pub type DieselPgPool = Pool<ConnectionManager<PgConnection>>;
pub type DieselPgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct AircraftRegistrationsRepository {
    pool: DieselPgPool,
    locations_repo: LocationsRepository,
}

impl AircraftRegistrationsRepository {
    pub fn new(pool: DieselPgPool) -> Self {
        let locations_repo = LocationsRepository::new(pool.clone());
        Self {
            pool,
            locations_repo,
        }
    }

    fn get_connection(&self) -> Result<DieselPgPooledConnection> {
        self.pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))
    }

    /// Upsert aircraft registrations into the database
    /// This will insert new aircraft registrations or update existing ones based on the primary key (registration_number)
    pub async fn upsert_aircraft_registrations<I>(&self, aircraft: I) -> Result<usize>
    where
        I: IntoIterator<Item = Aircraft>,
    {
        let mut conn = self.get_connection()?;
        let mut upserted_count = 0;

        // Process each aircraft for club linking and conversion
        let aircraft_vec: Vec<Aircraft> = aircraft.into_iter().collect();

        for aircraft_reg in aircraft_vec {
            // First, create/find location for this aircraft registration's address
            let location = match self
                .locations_repo
                .find_or_create(
                    aircraft_reg.street1.clone(),
                    aircraft_reg.street2.clone(),
                    aircraft_reg.city.clone(),
                    aircraft_reg.state.clone(),
                    aircraft_reg.zip_code.clone(),
                    aircraft_reg.region_code.clone(),
                    aircraft_reg.county_mail_code.clone(),
                    aircraft_reg.country_mail_code.clone(),
                    None, // geolocation will be set by triggers if coordinates are available
                )
                .await
            {
                Ok(location) => location,
                Err(e) => {
                    error!(
                        "Failed to create/find location for aircraft {}: {}",
                        aircraft_reg.n_number, e
                    );
                    // Continue processing other aircraft
                    continue;
                }
            };

            // Detect if this aircraft is registered to a soaring club and link it
            let club_id = if let Some(club_name) = aircraft_reg.club_name() {
                let clubs_repo = ClubsRepository::new(self.pool.clone());

                // Use the aircraft's registration location for the club
                let location_params = crate::clubs_repo::LocationParams {
                    street1: aircraft_reg.street1.clone(),
                    street2: aircraft_reg.street2.clone(),
                    city: aircraft_reg.city.clone(),
                    state: aircraft_reg.state.clone(),
                    zip_code: aircraft_reg.zip_code.clone(),
                    region_code: aircraft_reg.region_code.clone(),
                    county_mail_code: aircraft_reg.county_mail_code.clone(),
                    country_mail_code: aircraft_reg.country_mail_code.clone(),
                };

                match clubs_repo
                    .find_or_create_club(&club_name, location_params)
                    .await
                {
                    Ok(club) => {
                        info!(
                            "Linked aircraft {} to club: {} ({})",
                            aircraft_reg.n_number, club.name, club.id
                        );
                        Some(club.id)
                    }
                    Err(e) => {
                        warn!(
                            "Failed to find/create club '{}' for aircraft {}: {}",
                            club_name, aircraft_reg.n_number, e
                        );
                        None
                    }
                }
            } else {
                None
            };

            // Create NewAircraftRegistration with location_id and club_id
            let mut new_aircraft_reg: NewAircraftRegistration = aircraft_reg.clone().into();
            new_aircraft_reg.location_id = Some(location.id);
            new_aircraft_reg.club_id = club_id;
            let result = diesel::insert_into(aircraft_registrations::table)
                .values(&new_aircraft_reg)
                .on_conflict(aircraft_registrations::registration_number)
                .do_update()
                .set((
                    aircraft_registrations::serial_number
                        .eq(excluded(aircraft_registrations::serial_number)),
                    aircraft_registrations::manufacturer_code
                        .eq(excluded(aircraft_registrations::manufacturer_code)),
                    aircraft_registrations::model_code
                        .eq(excluded(aircraft_registrations::model_code)),
                    aircraft_registrations::series_code
                        .eq(excluded(aircraft_registrations::series_code)),
                    aircraft_registrations::engine_manufacturer_code
                        .eq(excluded(aircraft_registrations::engine_manufacturer_code)),
                    aircraft_registrations::engine_model_code
                        .eq(excluded(aircraft_registrations::engine_model_code)),
                    aircraft_registrations::year_mfr.eq(excluded(aircraft_registrations::year_mfr)),
                    aircraft_registrations::registrant_type_code
                        .eq(excluded(aircraft_registrations::registrant_type_code)),
                    aircraft_registrations::registrant_name
                        .eq(excluded(aircraft_registrations::registrant_name)),
                    aircraft_registrations::location_id
                        .eq(excluded(aircraft_registrations::location_id)),
                    aircraft_registrations::last_action_date
                        .eq(excluded(aircraft_registrations::last_action_date)),
                    aircraft_registrations::certificate_issue_date
                        .eq(excluded(aircraft_registrations::certificate_issue_date)),
                    aircraft_registrations::airworthiness_class
                        .eq(excluded(aircraft_registrations::airworthiness_class)),
                    aircraft_registrations::approved_operations_raw
                        .eq(excluded(aircraft_registrations::approved_operations_raw)),
                    aircraft_registrations::aircraft_type
                        .eq(excluded(aircraft_registrations::aircraft_type)),
                    aircraft_registrations::type_engine_code
                        .eq(excluded(aircraft_registrations::type_engine_code)),
                    aircraft_registrations::status_code
                        .eq(excluded(aircraft_registrations::status_code)),
                    aircraft_registrations::transponder_code
                        .eq(excluded(aircraft_registrations::transponder_code)),
                    aircraft_registrations::fractional_owner
                        .eq(excluded(aircraft_registrations::fractional_owner)),
                    aircraft_registrations::airworthiness_date
                        .eq(excluded(aircraft_registrations::airworthiness_date)),
                    aircraft_registrations::expiration_date
                        .eq(excluded(aircraft_registrations::expiration_date)),
                    aircraft_registrations::unique_id
                        .eq(excluded(aircraft_registrations::unique_id)),
                    aircraft_registrations::kit_mfr_name
                        .eq(excluded(aircraft_registrations::kit_mfr_name)),
                    aircraft_registrations::kit_model_name
                        .eq(excluded(aircraft_registrations::kit_model_name)),
                    aircraft_registrations::device_id
                        .eq(excluded(aircraft_registrations::device_id)),
                    aircraft_registrations::light_sport_type
                        .eq(excluded(aircraft_registrations::light_sport_type)),
                    aircraft_registrations::club_id.eq(excluded(aircraft_registrations::club_id)),
                ))
                .execute(&mut conn);

            match result {
                Ok(_) => {
                    // Now insert the other names into the aircraft_other_names table
                    if !aircraft_reg.other_names.is_empty() {
                        // First delete existing other names for this registration
                        let delete_result = diesel::delete(aircraft_other_names::table)
                            .filter(
                                aircraft_other_names::registration_number
                                    .eq(&aircraft_reg.n_number),
                            )
                            .execute(&mut conn);

                        if let Err(e) = delete_result {
                            warn!(
                                "Failed to delete existing other names for {}: {}",
                                aircraft_reg.n_number, e
                            );
                        }

                        // Insert new other names
                        for (seq, other_name) in aircraft_reg.other_names.iter().enumerate() {
                            let new_other_name = (
                                aircraft_other_names::registration_number
                                    .eq(&aircraft_reg.n_number),
                                aircraft_other_names::seq.eq((seq + 1) as i16), // 1-based sequence
                                aircraft_other_names::other_name.eq(other_name),
                            );

                            let insert_result = diesel::insert_into(aircraft_other_names::table)
                                .values(&new_other_name)
                                .execute(&mut conn);

                            if let Err(e) = insert_result {
                                warn!(
                                    "Failed to insert other name '{}' for {}: {}",
                                    other_name, aircraft_reg.n_number, e
                                );
                            }
                        }
                    }

                    // Now handle approved operations - delete existing and insert new ones
                    let approved_ops = aircraft_reg.to_approved_operations();
                    if !approved_ops.is_empty() {
                        // First delete existing approved operations for this registration
                        let delete_result = diesel::delete(aircraft_approved_operations::table)
                            .filter(
                                aircraft_approved_operations::aircraft_registration_id
                                    .eq(&aircraft_reg.n_number),
                            )
                            .execute(&mut conn);

                        if let Err(e) = delete_result {
                            warn!(
                                "Failed to delete existing approved operations for {}: {}",
                                aircraft_reg.n_number, e
                            );
                        }

                        // Insert new approved operations
                        let insert_result =
                            diesel::insert_into(aircraft_approved_operations::table)
                                .values(&approved_ops)
                                .execute(&mut conn);

                        if let Err(e) = insert_result {
                            warn!(
                                "Failed to insert approved operations for {}: {}",
                                aircraft_reg.n_number, e
                            );
                        }
                    }

                    upserted_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to upsert aircraft registration {}: {}",
                        new_aircraft_reg.registration_number, e
                    );
                    // Continue with other aircraft rather than failing the entire batch
                }
            }
        }

        info!(
            "Successfully upserted {} aircraft registrations",
            upserted_count
        );
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
    /// Helper method to fetch other names for an aircraft registration
    async fn get_other_names(&self, registration_number: &str) -> Result<Vec<String>> {
        let mut conn = self.get_connection()?;
        let other_names = aircraft_other_names::table
            .filter(aircraft_other_names::registration_number.eq(registration_number))
            .order_by(aircraft_other_names::seq)
            .select(aircraft_other_names::other_name)
            .load::<String>(&mut conn)?;
        Ok(other_names)
    }

    /// Convert an AircraftRegistrationModel to Aircraft, including other_names
    async fn model_to_aircraft(&self, model: AircraftRegistrationModel) -> Result<Aircraft> {
        let other_names = self.get_other_names(&model.registration_number).await?;
        let mut aircraft: Aircraft = model.into();
        aircraft.other_names = other_names;
        Ok(aircraft)
    }

    pub async fn get_aircraft_registration_by_n_number(
        &self,
        registration_number: &str,
    ) -> Result<Option<Aircraft>> {
        let mut conn = self.get_connection()?;
        let aircraft_model = aircraft_registrations::table
            .filter(aircraft_registrations::registration_number.eq(registration_number))
            .select(AircraftRegistrationModel::as_select())
            .first::<AircraftRegistrationModel>(&mut conn)
            .optional()?;

        match aircraft_model {
            Some(model) => {
                let aircraft = self.model_to_aircraft(model).await?;
                Ok(Some(aircraft))
            }
            None => Ok(None),
        }
    }

    /// Search aircraft registrations by registrant name
    pub async fn search_by_registrant_name(&self, registrant_name: &str) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;
        let search_pattern = format!("%{}%", registrant_name);
        let aircraft_models = aircraft_registrations::table
            .filter(aircraft_registrations::registrant_name.ilike(&search_pattern))
            .select(AircraftRegistrationModel::as_select())
            .load::<AircraftRegistrationModel>(&mut conn)?;

        let mut aircraft_list = Vec::new();
        for model in aircraft_models {
            let aircraft = self.model_to_aircraft(model).await?;
            aircraft_list.push(aircraft);
        }

        Ok(aircraft_list)
    }

    /// Search aircraft registrations by transponder code
    pub async fn search_by_transponder_code(&self, transponder_code: u32) -> Result<Vec<Aircraft>> {
        let mut conn = self.get_connection()?;
        let transponder_code_i64 = transponder_code as i64;
        let aircraft_models = aircraft_registrations::table
            .filter(aircraft_registrations::transponder_code.eq(transponder_code_i64))
            .select(AircraftRegistrationModel::as_select())
            .load::<AircraftRegistrationModel>(&mut conn)?;

        let mut aircraft_list = Vec::new();
        for model in aircraft_models {
            let aircraft = self.model_to_aircraft(model).await?;
            aircraft_list.push(aircraft);
        }

        Ok(aircraft_list)
    }

    // TODO: The following methods were removed because club_id moved from aircraft_registrations
    // to devices table. To query aircraft by club, you should now:
    // 1. Query devices table for devices with the given club_id
    // 2. Join with aircraft_registrations on device_id
    //
    // Removed methods:
    // - get_by_club_id
    // - get_aircraft_models_by_club_id
    // - get_aircraft_with_models_by_club_id

    /// Update is_tow_plane field for an aircraft based on device_id
    /// Only updates if the current value is different to avoid updating the updated_at column unnecessarily
    pub async fn update_tow_plane_status_by_device_id(
        &self,
        device_id: Uuid,
        is_tow_plane: bool,
    ) -> Result<bool> {
        let mut conn = self.get_connection()?;

        // First check current value to avoid unnecessary updates
        let current_value = aircraft_registrations::table
            .filter(aircraft_registrations::device_id.eq(device_id))
            .select(aircraft_registrations::is_tow_plane)
            .first::<Option<bool>>(&mut conn)
            .optional()?;

        match current_value {
            Some(Some(current)) if current == is_tow_plane => {
                // Value is already correct, no update needed
                Ok(false)
            }
            Some(_) | None => {
                // Value is different or row doesn't exist, perform update
                let updated_count = diesel::update(aircraft_registrations::table)
                    .filter(aircraft_registrations::device_id.eq(device_id))
                    .set(aircraft_registrations::is_tow_plane.eq(Some(is_tow_plane)))
                    .execute(&mut conn)?;

                Ok(updated_count > 0)
            }
        }
    }

    /// Get aircraft registration by device ID
    pub async fn get_aircraft_registration_by_device_id(
        &self,
        device_id: Uuid,
    ) -> Result<Option<AircraftRegistrationModel>> {
        let mut conn = self.get_connection()?;
        let aircraft_model = aircraft_registrations::table
            .filter(aircraft_registrations::device_id.eq(device_id))
            .select(AircraftRegistrationModel::as_select())
            .first::<AircraftRegistrationModel>(&mut conn)
            .optional()?;

        Ok(aircraft_model)
    }

    /// Get aircraft registration model by registration number (N-number)
    /// Returns the model directly without other_names
    pub async fn get_aircraft_registration_model_by_n_number(
        &self,
        registration_number: &str,
    ) -> Result<Option<AircraftRegistrationModel>> {
        let mut conn = self.get_connection()?;
        let aircraft_model = aircraft_registrations::table
            .filter(aircraft_registrations::registration_number.eq(registration_number))
            .select(AircraftRegistrationModel::as_select())
            .first::<AircraftRegistrationModel>(&mut conn)
            .optional()?;

        Ok(aircraft_model)
    }

    /// Get aircraft registrations for multiple device IDs (batch query)
    #[tracing::instrument(skip(self, device_ids), fields(device_count = device_ids.len()))]
    pub async fn get_aircraft_registrations_by_device_ids(
        &self,
        device_ids: &[Uuid],
    ) -> Result<Vec<AircraftRegistrationModel>> {
        if device_ids.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            "Querying aircraft registrations for {} devices",
            device_ids.len()
        );
        let mut conn = self.get_connection()?;
        let registrations = aircraft_registrations::table
            .filter(aircraft_registrations::device_id.eq_any(device_ids))
            .select(AircraftRegistrationModel::as_select())
            .load::<AircraftRegistrationModel>(&mut conn)?;

        tracing::info!("Found {} aircraft registrations", registrations.len());
        Ok(registrations)
    }
}
