use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::upsert::excluded;
use futures_util::stream::{self, StreamExt};
use tracing::{info, warn};
use uuid::Uuid;

use crate::aircraft_registrations::{
    Aircraft, AircraftRegistrationModel, NewAircraftOtherName, NewAircraftRegistration,
};
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

        let aircraft_vec: Vec<Aircraft> = aircraft.into_iter().collect();
        let total_count = aircraft_vec.len();
        info!("Starting import of {} aircraft registrations", total_count);

        let start_time = Instant::now();

        // PHASE 1: Build club cache
        info!("Building club cache...");

        // Build club cache - key is club_name
        let mut club_cache: HashMap<String, Uuid> = HashMap::new();

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

        // PHASE 2: Fast bulk insert of all aircraft registrations (WITHOUT locations)
        // PostgreSQL has a limit of 65535 parameters per query
        // With ~30 fields per aircraft, we can safely do ~2000 records per batch
        const BATCH_SIZE: usize = 2000;
        let mut upserted_count = 0;

        info!("Phase 2a: Bulk inserting all aircraft registrations without locations...");

        for batch_start in (0..total_count).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(total_count);
            let batch = &aircraft_vec[batch_start..batch_end];

            let mut conn = self.get_connection()?;

            let mut aircraft_registrations: Vec<NewAircraftRegistration> = Vec::new();
            let mut all_approved_ops: Vec<
                crate::aircraft_registrations::NewAircraftApprovedOperation,
            > = Vec::new();
            let mut all_other_names: Vec<NewAircraftOtherName> = Vec::new();

            for aircraft_reg in batch.iter() {
                // Look up club from cache
                let club_id = aircraft_reg
                    .club_name()
                    .and_then(|club_name| club_cache.get(&club_name).copied());

                // Create NewAircraftRegistration with NULL location_id
                let mut new_aircraft_reg: NewAircraftRegistration = aircraft_reg.clone().into();
                new_aircraft_reg.location_id = None; // Will fill in Phase 2b
                new_aircraft_reg.club_id = club_id;

                aircraft_registrations.push(new_aircraft_reg);

                // Collect approved operations
                for op in aircraft_reg.to_approved_operations() {
                    all_approved_ops.push(op);
                }

                // Collect other names
                for other_name in aircraft_reg.to_other_names() {
                    all_other_names.push(other_name);
                }
            }

            // Batch upsert aircraft registrations - INSERT ALL at once
            let result = diesel::insert_into(aircraft_registrations::table)
                .values(&aircraft_registrations)
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
                Ok(count) => upserted_count += count,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to batch upsert aircraft at batch {}-{}: {}",
                        batch_start,
                        batch_end,
                        e
                    ));
                }
            }

            // Batch insert approved_operations
            if !all_approved_ops.is_empty() {
                let registration_numbers: Vec<String> =
                    batch.iter().map(|a| a.n_number.clone()).collect();

                // Delete existing approved_operations for this batch
                let _ = diesel::delete(aircraft_approved_operations::table)
                    .filter(
                        aircraft_approved_operations::aircraft_registration_id
                            .eq_any(&registration_numbers),
                    )
                    .execute(&mut conn);

                // Insert new approved_operations
                let _ = diesel::insert_into(aircraft_approved_operations::table)
                    .values(&all_approved_ops)
                    .execute(&mut conn);
            }

            // Batch insert aircraft_other_names
            if !all_other_names.is_empty() {
                let registration_numbers: Vec<String> =
                    batch.iter().map(|a| a.n_number.clone()).collect();

                // Delete existing other_names for this batch
                let _ = diesel::delete(aircraft_other_names::table)
                    .filter(aircraft_other_names::registration_number.eq_any(&registration_numbers))
                    .execute(&mut conn);

                // Insert new other_names
                let _ = diesel::insert_into(aircraft_other_names::table)
                    .values(&all_other_names)
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

        info!(
            "Phase 2a complete: Inserted {} aircraft registrations in {:.1} seconds",
            upserted_count,
            start_time.elapsed().as_secs_f64()
        );

        // PHASE 2b: Fill in location_id for aircraft with address data
        info!("Phase 2b: Filling in locations for all aircraft...");
        let phase2b_start = Instant::now();

        // Use original aircraft_vec which has address fields
        // Process in batches with parallel location lookups
        const LOCATION_BATCH_SIZE: usize = 1000;
        const MAX_CONCURRENT: usize = 50; // Higher concurrency for location lookups only
        let mut locations_filled = 0;
        let mut total_location_lookup_time = 0.0;
        let mut total_update_time = 0.0;

        // Cache to avoid duplicate location lookups - key is (street1, street2, city, state, zip, country)
        let mut location_cache: HashMap<
            (String, String, String, String, String, String),
            uuid::Uuid,
        > = HashMap::new();
        let mut cache_hits = 0;
        let mut cache_misses = 0;

        for batch_start in (0..total_count).step_by(LOCATION_BATCH_SIZE) {
            let batch_end = (batch_start + LOCATION_BATCH_SIZE).min(total_count);
            let batch = &aircraft_vec[batch_start..batch_end];

            // Parallel location lookups - TIMED (only for cache misses)
            let lookup_start = Instant::now();

            // First, check which addresses need lookup (not in cache)
            let mut to_lookup: Vec<&Aircraft> = Vec::new();
            let mut cached_locations: Vec<(String, uuid::Uuid)> = Vec::new();

            for aircraft_reg in batch.iter() {
                let cache_key = (
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

                if let Some(&location_id) = location_cache.get(&cache_key) {
                    // Cache hit - use cached location
                    cached_locations.push((aircraft_reg.n_number.clone(), location_id));
                    cache_hits += 1;
                } else {
                    // Cache miss - need to look up
                    to_lookup.push(aircraft_reg);
                    cache_misses += 1;
                }
            }

            // Only do database lookups for cache misses
            let location_results: Vec<(String, Result<crate::locations::Location>)> =
                stream::iter(to_lookup.iter().map(|aircraft_reg| {
                    let reg_num = aircraft_reg.n_number.clone();
                    let locations_repo = self.locations_repo.clone();
                    let street1 = aircraft_reg.street1.clone();
                    let street2 = aircraft_reg.street2.clone();
                    let city = aircraft_reg.city.clone();
                    let state = aircraft_reg.state.clone();
                    let zip = aircraft_reg.zip_code.clone();
                    let region = aircraft_reg.region_code.clone();
                    let country = aircraft_reg.country_mail_code.clone();
                    async move {
                        let result = locations_repo
                            .find_or_create(
                                street1, street2, city, state, zip, region, country, None,
                            )
                            .await;
                        (reg_num, result)
                    }
                }))
                .buffer_unordered(MAX_CONCURRENT)
                .collect()
                .await;
            let lookup_elapsed = lookup_start.elapsed().as_secs_f64();
            total_location_lookup_time += lookup_elapsed;

            // Add newly looked-up locations to cache and prepare for update
            let cache_hit_count = cached_locations.len();
            let mut all_updates: Vec<(String, uuid::Uuid)> = cached_locations;
            for (i, (reg_num, location_result)) in location_results.into_iter().enumerate() {
                if let Ok(location) = location_result {
                    // Add to cache
                    let aircraft_reg = to_lookup[i];
                    let cache_key = (
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
                    location_cache.insert(cache_key, location.id);

                    all_updates.push((reg_num, location.id));
                }
            }

            // Update location_ids in database - TIMED
            let update_start = Instant::now();
            let mut conn = self.get_connection()?;
            for (reg_num, location_id) in all_updates {
                match diesel::update(aircraft_registrations::table)
                    .filter(aircraft_registrations::registration_number.eq(&reg_num))
                    .set(aircraft_registrations::location_id.eq(location_id))
                    .execute(&mut conn)
                {
                    Ok(_) => locations_filled += 1,
                    Err(e) => warn!("Failed to update location for {}: {}", reg_num, e),
                }
            }
            let update_elapsed = update_start.elapsed().as_secs_f64();
            total_update_time += update_elapsed;

            if batch_end.is_multiple_of(1000) || batch_end == total_count {
                info!(
                    "Location progress: {}/{} ({:.1}%) | Lookups: {} | Cache hits: {} | Batch time: {:.2}s (lookup: {:.2}s, update: {:.2}s)",
                    batch_end,
                    total_count,
                    (batch_end as f64 / total_count as f64) * 100.0,
                    to_lookup.len(),
                    cache_hit_count,
                    lookup_elapsed + update_elapsed,
                    lookup_elapsed,
                    update_elapsed
                );
            }
        }

        info!(
            "Phase 2b complete: Filled {} locations in {:.1} seconds (Lookups: {:.1}s, Updates: {:.1}s) | Cache: {} hits, {} misses ({:.1}% hit rate)",
            locations_filled,
            phase2b_start.elapsed().as_secs_f64(),
            total_location_lookup_time,
            total_update_time,
            cache_hits,
            cache_misses,
            if (cache_hits + cache_misses) > 0 {
                (cache_hits as f64 / (cache_hits + cache_misses) as f64) * 100.0
            } else {
                0.0
            }
        );

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
