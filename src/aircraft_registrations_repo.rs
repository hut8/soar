use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use tracing::{info, warn};
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
        use std::collections::HashMap;
        use std::time::Instant;

        // Type aliases to reduce complexity
        type LocationKey = (String, String, String, String, String, String);
        type LocationData = (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        );

        let aircraft_vec: Vec<Aircraft> = aircraft.into_iter().collect();
        let total_count = aircraft_vec.len();
        info!("Starting import of {} aircraft registrations", total_count);

        let start_time = Instant::now();

        // PHASE 1: Build caches for locations and clubs
        info!("Building location and club caches...");

        // Build location cache - key is (street1, street2, city, state, zip, country)
        let mut location_cache: HashMap<LocationKey, Uuid> = HashMap::new();

        // Build club cache - key is club_name
        let mut club_cache: HashMap<String, Uuid> = HashMap::new();

        // Collect unique locations
        let mut unique_locations: Vec<LocationData> = Vec::new();
        let mut location_set: std::collections::HashSet<LocationKey> =
            std::collections::HashSet::new();

        for aircraft_reg in &aircraft_vec {
            let key = (
                aircraft_reg.street1.clone().unwrap_or_default(),
                aircraft_reg.street2.clone().unwrap_or_default(),
                aircraft_reg.city.clone().unwrap_or_default(),
                aircraft_reg.state.clone().unwrap_or_default(),
                aircraft_reg.zip_code.clone().unwrap_or_default(),
                aircraft_reg
                    .country_mail_code
                    .clone()
                    .unwrap_or_else(|| "US".to_string()),
            );

            if location_set.insert(key.clone()) {
                unique_locations.push((
                    aircraft_reg.street1.clone().unwrap_or_default(),
                    aircraft_reg.street2.clone().unwrap_or_default(),
                    aircraft_reg.city.clone().unwrap_or_default(),
                    aircraft_reg.state.clone().unwrap_or_default(),
                    aircraft_reg.zip_code.clone().unwrap_or_default(),
                    aircraft_reg.region_code.clone().unwrap_or_default(),
                    aircraft_reg.county_mail_code.clone().unwrap_or_default(),
                    aircraft_reg
                        .country_mail_code
                        .clone()
                        .unwrap_or_else(|| "US".to_string()),
                ));
            }
        }

        info!("Found {} unique locations to cache", unique_locations.len());

        // Batch create all unique locations and populate cache
        for loc in unique_locations {
            match self
                .locations_repo
                .find_or_create(
                    if loc.0.is_empty() {
                        None
                    } else {
                        Some(loc.0.clone())
                    },
                    if loc.1.is_empty() {
                        None
                    } else {
                        Some(loc.1.clone())
                    },
                    if loc.2.is_empty() {
                        None
                    } else {
                        Some(loc.2.clone())
                    },
                    if loc.3.is_empty() {
                        None
                    } else {
                        Some(loc.3.clone())
                    },
                    if loc.4.is_empty() {
                        None
                    } else {
                        Some(loc.4.clone())
                    },
                    if loc.5.is_empty() {
                        None
                    } else {
                        Some(loc.5.clone())
                    },
                    if loc.6.is_empty() {
                        None
                    } else {
                        Some(loc.6.clone())
                    },
                    Some(loc.7.clone()),
                    None,
                )
                .await
            {
                Ok(location) => {
                    let key = (loc.0, loc.1, loc.2, loc.3, loc.4, loc.7);
                    location_cache.insert(key, location.id);
                }
                Err(e) => {
                    warn!("Failed to create location: {}", e);
                }
            }
        }

        info!("Location cache built with {} entries", location_cache.len());

        // Collect unique clubs
        let mut unique_clubs: Vec<(String, crate::clubs_repo::LocationParams)> = Vec::new();
        let mut club_set: std::collections::HashSet<String> = std::collections::HashSet::new();

        for aircraft_reg in &aircraft_vec {
            if let Some(club_name) = aircraft_reg.club_name()
                && club_set.insert(club_name.clone())
            {
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
                unique_clubs.push((club_name, location_params));
            }
        }

        info!("Found {} unique clubs to cache", unique_clubs.len());

        // Batch create all unique clubs and populate cache
        let clubs_repo = ClubsRepository::new(self.pool.clone());
        for (club_name, location_params) in unique_clubs {
            match clubs_repo
                .find_or_create_club(&club_name, location_params)
                .await
            {
                Ok(club) => {
                    club_cache.insert(club_name.clone(), club.id);
                }
                Err(e) => {
                    warn!("Failed to create club '{}': {}", club_name, e);
                }
            }
        }

        info!("Club cache built with {} entries", club_cache.len());

        // PHASE 2: Process aircraft in batches
        const BATCH_SIZE: usize = 5000;
        let mut upserted_count = 0;

        for batch_start in (0..total_count).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(total_count);
            let batch = &aircraft_vec[batch_start..batch_end];

            let mut conn = self.get_connection()?;

            // Prepare batch data
            let mut aircraft_registrations: Vec<NewAircraftRegistration> = Vec::new();
            let mut all_other_names: Vec<(String, i16, String)> = Vec::new();
            let mut all_approved_ops: Vec<
                crate::aircraft_registrations::NewAircraftApprovedOperation,
            > = Vec::new();
            let mut registration_numbers: Vec<String> = Vec::new();

            for aircraft_reg in batch {
                // Look up location from cache
                let location_key = (
                    aircraft_reg.street1.clone().unwrap_or_default(),
                    aircraft_reg.street2.clone().unwrap_or_default(),
                    aircraft_reg.city.clone().unwrap_or_default(),
                    aircraft_reg.state.clone().unwrap_or_default(),
                    aircraft_reg.zip_code.clone().unwrap_or_default(),
                    aircraft_reg
                        .country_mail_code
                        .clone()
                        .unwrap_or_else(|| "US".to_string()),
                );

                let location_id = location_cache.get(&location_key).copied();

                // Look up club from cache
                let club_id = aircraft_reg
                    .club_name()
                    .and_then(|club_name| club_cache.get(&club_name).copied());

                // Create NewAircraftRegistration
                let mut new_aircraft_reg: NewAircraftRegistration = aircraft_reg.clone().into();
                new_aircraft_reg.location_id = location_id;
                new_aircraft_reg.club_id = club_id;

                aircraft_registrations.push(new_aircraft_reg);
                registration_numbers.push(aircraft_reg.n_number.clone());

                // Collect other names
                for (seq, other_name) in aircraft_reg.other_names.iter().enumerate() {
                    all_other_names.push((
                        aircraft_reg.n_number.clone(),
                        (seq + 1) as i16,
                        other_name.clone(),
                    ));
                }

                // Collect approved operations
                for op in aircraft_reg.to_approved_operations() {
                    all_approved_ops.push(op);
                }
            }

            // Batch upsert aircraft registrations
            for new_aircraft_reg in &aircraft_registrations {
                let result = diesel::insert_into(aircraft_registrations::table)
                    .values(new_aircraft_reg)
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
                        aircraft_registrations::year_mfr
                            .eq(excluded(aircraft_registrations::year_mfr)),
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
                        aircraft_registrations::club_id
                            .eq(excluded(aircraft_registrations::club_id)),
                    ))
                    .execute(&mut conn);

                match result {
                    Ok(_) => upserted_count += 1,
                    Err(e) => warn!(
                        "Failed to upsert aircraft {}: {}",
                        new_aircraft_reg.registration_number, e
                    ),
                }
            }

            // Batch delete existing other_names for this batch
            if !registration_numbers.is_empty() {
                let _ = diesel::delete(aircraft_other_names::table)
                    .filter(aircraft_other_names::registration_number.eq_any(&registration_numbers))
                    .execute(&mut conn);
            }

            // Batch insert other_names
            if !all_other_names.is_empty() {
                for (reg_num, seq, other_name) in &all_other_names {
                    let _ = diesel::insert_into(aircraft_other_names::table)
                        .values((
                            aircraft_other_names::registration_number.eq(reg_num),
                            aircraft_other_names::seq.eq(seq),
                            aircraft_other_names::other_name.eq(other_name),
                        ))
                        .execute(&mut conn);
                }
            }

            // Batch delete existing approved_operations for this batch
            if !registration_numbers.is_empty() {
                let _ = diesel::delete(aircraft_approved_operations::table)
                    .filter(
                        aircraft_approved_operations::aircraft_registration_id
                            .eq_any(&registration_numbers),
                    )
                    .execute(&mut conn);
            }

            // Batch insert approved_operations
            if !all_approved_ops.is_empty() {
                let _ = diesel::insert_into(aircraft_approved_operations::table)
                    .values(&all_approved_ops)
                    .execute(&mut conn);
            }

            // Log progress every 1000 records
            if batch_end.is_multiple_of(1000) || batch_end == total_count {
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = batch_end as f64 / elapsed;
                let remaining = total_count - batch_end;
                let eta_seconds = (remaining as f64 / rate) as u64;
                let percentage = (batch_end as f64 / total_count as f64) * 100.0;

                info!(
                    "Progress: {}/{} ({:.1}%) | Rate: {:.0} records/sec | ETA: {}m {}s",
                    batch_end,
                    total_count,
                    percentage,
                    rate,
                    eta_seconds / 60,
                    eta_seconds % 60
                );
            }
        }

        let elapsed = start_time.elapsed();
        info!(
            "Successfully upserted {} aircraft registrations in {:.1} seconds ({:.0} records/sec)",
            upserted_count,
            elapsed.as_secs_f64(),
            upserted_count as f64 / elapsed.as_secs_f64()
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
