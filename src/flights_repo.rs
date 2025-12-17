use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::flights::{Flight, FlightModel, TimeoutPhase};
use crate::web::PgPool;

#[derive(Clone)]
pub struct FlightsRepository {
    pool: PgPool,
}

impl FlightsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new flight and insert it into the database
    pub async fn create_flight(&self, flight: Flight) -> Result<()> {
        self.insert_flight(&flight).await
    }

    /// Insert a new flight into the database
    pub async fn insert_flight(&self, flight: &Flight) -> Result<()> {
        use crate::schema::flights;

        let pool = self.pool.clone();
        let flight_model: FlightModel = flight.clone().into();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::insert_into(flights::table)
                .values(&flight_model)
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    /// Update flight with landing information
    #[allow(clippy::too_many_arguments)]
    pub async fn update_flight_landing(
        &self,
        flight_id: Uuid,
        landing_time_param: DateTime<Utc>,
        arrival_airport_id_param: Option<i32>,
        landing_location_id_param: Option<Uuid>,
        end_location_id_param: Option<Uuid>,
        landing_altitude_offset_ft_param: Option<i32>,
        landing_runway_ident_param: Option<String>,
        total_distance_meters_param: Option<f64>,
        maximum_displacement_meters_param: Option<f64>,
        runways_inferred_param: Option<bool>,
        last_fix_at_param: Option<DateTime<Utc>>,
    ) -> Result<()> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // If last_fix_at not provided, use landing_time (by definition a flight has at least one fix)
            let last_fix_time = last_fix_at_param.unwrap_or(landing_time_param);

            // Single UPDATE query with all fields including last_fix_at and end_location_id
            let rows_affected = diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    landing_time.eq(&Some(landing_time_param)),
                    arrival_airport_id.eq(&arrival_airport_id_param),
                    landing_location_id.eq(&landing_location_id_param),
                    end_location_id.eq(&end_location_id_param),
                    landing_altitude_offset_ft.eq(&landing_altitude_offset_ft_param),
                    landing_runway_ident.eq(&landing_runway_ident_param),
                    total_distance_meters.eq(&total_distance_meters_param),
                    maximum_displacement_meters.eq(&maximum_displacement_meters_param),
                    runways_inferred.eq(&runways_inferred_param),
                    last_fix_at.eq(last_fix_time),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            if rows_affected == 0 {
                return Err(anyhow::anyhow!(
                    "No rows affected when updating flight landing"
                ));
            }

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    /// Get a flight by its ID
    pub async fn get_flight_by_id(&self, flight_id: Uuid) -> Result<Option<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_model: Option<FlightModel> = flights
                .filter(id.eq(flight_id))
                .select(FlightModel::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<FlightModel>, anyhow::Error>(flight_model)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Get all flights for a specific device, ordered by takeoff time descending
    pub async fn get_flights_for_device(&self, device_id_val: &uuid::Uuid) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();
        let device_id_val = *device_id_val;

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(aircraft_id.eq(device_id_val))
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightModel>, anyhow::Error>(flight_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    pub async fn get_flights_for_device_paginated(
        &self,
        device_id_val: &uuid::Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Flight>, i64)> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();
        let device_id_val = *device_id_val;

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Get total count
            let total_count = flights
                .filter(aircraft_id.eq(device_id_val))
                .count()
                .get_result::<i64>(&mut conn)?;

            // Get paginated results
            let offset = (page - 1) * per_page;
            let flight_models: Vec<FlightModel> = flights
                .filter(aircraft_id.eq(device_id_val))
                .order(takeoff_time.desc())
                .limit(per_page)
                .offset(offset)
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<(Vec<FlightModel>, i64), anyhow::Error>((flight_models, total_count))
        })
        .await??;

        let (flight_models, total_count) = results;
        Ok((
            flight_models
                .into_iter()
                .map(|model| model.into())
                .collect(),
            total_count,
        ))
    }

    /// Get flights in progress (no landing time and not timed out) ordered by latest fix time descending
    /// Limited to the specified number of flights with optional offset
    pub async fn get_flights_in_progress(&self, limit: i64, offset: i64) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(landing_time.is_null().and(timed_out_at.is_null()))
                .order(last_fix_at.desc())
                .limit(limit)
                .offset(offset)
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightModel>, anyhow::Error>(flight_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Get active flights for a device (no landing_time and no timed_out_at)
    /// Returns only flights that are truly active (not completed or timed out)
    pub async fn get_active_flights_for_device(&self, device_id_val: Uuid) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(aircraft_id.eq(device_id_val))
                .filter(landing_time.is_null())
                .filter(timed_out_at.is_null())
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightModel>, anyhow::Error>(flight_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Find the most recent timed-out flight for a device that timed out within the last 12 hours.
    ///
    /// This is used for "flight coalescing" to handle aircraft that temporarily go out of receiver range.
    /// Scenario: An aircraft is tracked, flies out of range (e.g., trans-atlantic flight), then comes
    /// back into range. Without coalescing, this would create two separate flights (the first timed out,
    /// the second starting mid-flight). With coalescing, we resume tracking the original flight.
    ///
    /// The 12-hour window distinguishes between:
    /// - "Temporarily out of receiver range" (< 12 hours) → resume the same flight
    /// - "Out of range for so long they likely landed and took off again" (> 12 hours) → create new flight
    ///
    /// Returns Some(flight) if:
    /// - The most recent flight for this device has timed_out_at set
    /// - AND timed_out_at is within the last 12 hours
    ///
    /// Otherwise returns None.
    pub async fn find_recent_timed_out_flight(
        &self,
        device_id_val: Uuid,
    ) -> Result<Option<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Get the most recent flight for this device (by last_fix_at, which is always set)
            let most_recent_flight: Option<FlightModel> = flights
                .filter(aircraft_id.eq(device_id_val))
                .order(last_fix_at.desc())
                .select(FlightModel::as_select())
                .first(&mut conn)
                .optional()?;

            // Check if it's timed out and within 12 hours
            if let Some(flight_model) = most_recent_flight
                && let Some(timed_out_time) = flight_model.timed_out_at
            {
                let now = Utc::now();
                let elapsed = now.signed_duration_since(timed_out_time);
                let twelve_hours = chrono::Duration::hours(12);

                if elapsed < twelve_hours {
                    return Ok::<Option<FlightModel>, anyhow::Error>(Some(flight_model));
                }
            }

            Ok(None)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Get recent completed flights (with landing time OR timeout) ordered by completion time descending
    /// Completed means either landed (landing_time is set) or timed out (timed_out_at is set)
    pub async fn get_completed_flights(&self, limit: i64, offset: i64) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(landing_time.is_not_null().or(timed_out_at.is_not_null()))
                .order(landing_time.desc().nulls_last())
                .limit(limit)
                .offset(offset)
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightModel>, anyhow::Error>(flight_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Get flights within a time range, optionally filtered by device
    pub async fn get_flights_in_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        device_address: Option<&str>,
    ) -> Result<Vec<Flight>> {
        if let Some(device_address) = device_address {
            self.get_flights_in_time_range_for_device(start_time, end_time, device_address)
                .await
        } else {
            self.get_flights_in_time_range_all(start_time, end_time)
                .await
        }
    }

    /// Get flights within a time range for a specific device
    async fn get_flights_in_time_range_for_device(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        device_address_param: &str,
    ) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();
        let device_address_val = device_address_param.to_string();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(device_address.eq(&device_address_val))
                .filter(takeoff_time.ge(&start_time))
                .filter(takeoff_time.le(&end_time))
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightModel>, anyhow::Error>(flight_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Get all flights within a time range
    async fn get_flights_in_time_range_all(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(takeoff_time.ge(&start_time))
                .filter(takeoff_time.le(&end_time))
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightModel>, anyhow::Error>(flight_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Get the total count of flights in the database
    pub async fn get_flight_count(&self) -> Result<i64> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let count = flights.count().get_result::<i64>(&mut conn)?;

            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(count)
    }

    /// Get the count of flights in progress (no landing time and not timed out)
    pub async fn get_flights_in_progress_count(&self) -> Result<i64> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let count = flights
                .filter(landing_time.is_null().and(timed_out_at.is_null()))
                .count()
                .get_result::<i64>(&mut conn)?;

            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(count)
    }

    /// Get the count of completed flights (with landing time OR timeout)
    pub async fn get_completed_flights_count(&self) -> Result<i64> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let count = flights
                .filter(landing_time.is_not_null().or(timed_out_at.is_not_null()))
                .count()
                .get_result::<i64>(&mut conn)?;

            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(count)
    }

    /// Update towing information for a glider flight
    pub async fn update_towing_info(
        &self,
        glider_flight_id: Uuid,
        towplane_device_id: Uuid,
        towplane_flight_id: Uuid,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(glider_flight_id)))
                .set((
                    towed_by_aircraft_id.eq(Some(towplane_device_id)),
                    towed_by_flight_id.eq(Some(towplane_flight_id)),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Update tow release information for a glider flight
    /// Calculates tow_release_height_delta_ft by querying the towplane flight's first fix
    pub async fn update_tow_release(
        &self,
        glider_flight_id: Uuid,
        release_altitude_ft: i32,
        release_time: DateTime<Utc>,
    ) -> Result<bool> {
        use crate::schema::fixes::dsl as fixes_dsl;
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // First, get the glider flight's towplane flight ID
            let towplane_flight_id_opt: Option<Uuid> = flights
                .filter(id.eq(glider_flight_id))
                .select(towed_by_flight_id)
                .first(&mut conn)?;

            // Calculate tow release height delta if we have a towplane flight
            let height_delta = if let Some(towplane_flight_id) = towplane_flight_id_opt {
                // Get the towplane flight's time range for partition pruning
                let flight_times: Option<(chrono::DateTime<Utc>, chrono::DateTime<Utc>)> = flights
                    .filter(id.eq(towplane_flight_id))
                    .select((created_at, last_fix_at))
                    .first(&mut conn)
                    .optional()?;

                // Get the first fix of the towplane flight (chronologically) - just need altitude
                let first_fix_altitude: Option<Option<i32>> = if let Some((start_time, end_time)) =
                    flight_times
                {
                    // Add 1-hour buffer for partition pruning
                    let start_with_buffer = start_time - chrono::Duration::hours(1);
                    let end_with_buffer = end_time + chrono::Duration::hours(1);

                    fixes_dsl::fixes
                        .filter(fixes_dsl::flight_id.eq(towplane_flight_id))
                        .filter(fixes_dsl::received_at.between(start_with_buffer, end_with_buffer))
                        .order_by(fixes_dsl::received_at.asc())
                        .select(fixes_dsl::altitude_msl_feet)
                        .first(&mut conn)
                        .optional()?
                } else {
                    None
                };

                // Calculate delta: release altitude - towplane takeoff altitude
                first_fix_altitude.and_then(|alt_opt| {
                    alt_opt.map(|takeoff_alt| release_altitude_ft - takeoff_alt)
                })
            } else {
                None
            };

            // Update the glider flight with release info and calculated delta
            let rows = diesel::update(flights.filter(id.eq(glider_flight_id)))
                .set((
                    tow_release_altitude_msl_ft.eq(Some(release_altitude_ft)),
                    tow_release_time.eq(Some(release_time)),
                    tow_release_height_delta_ft.eq(height_delta),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Delete a flight by ID
    pub async fn delete_flight(&self, flight_id: Uuid) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::delete(flights.filter(id.eq(flight_id))).execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Get flights associated with an airport (either departure or arrival) within a time range
    pub async fn get_flights_by_airport(
        &self,
        airport_id_val: i32,
        since: DateTime<Utc>,
    ) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models = flights
                .filter(
                    departure_airport_id
                        .eq(Some(airport_id_val))
                        .or(arrival_airport_id.eq(Some(airport_id_val))),
                )
                .filter(takeoff_time.ge(Some(since)))
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load::<FlightModel>(&mut conn)?;

            let result_flights: Vec<Flight> = flight_models.into_iter().map(|f| f.into()).collect();

            Ok::<Vec<Flight>, anyhow::Error>(result_flights)
        })
        .await??;

        Ok(results)
    }

    /// Get flights for a specific club with optional date and completion filters
    /// If date is provided (YYYYMMDD format), filters flights to that specific date
    /// If completed is Some(true), returns only completed flights (with landing_time OR timed_out_at)
    /// If completed is Some(false), returns only in-progress flights (no landing_time and no timed_out_at)
    /// If completed is None, returns all flights
    /// Always returns flights in most recent first order (by takeoff_time descending)
    pub async fn get_flights_by_club(
        &self,
        club_id_val: Uuid,
        date: Option<chrono::NaiveDate>,
        completed: Option<bool>,
    ) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Start with base query filtering by club_id
            let mut query = flights.filter(club_id.eq(Some(club_id_val))).into_boxed();

            // Apply date filter if provided
            if let Some(date_val) = date {
                let start_of_day = date_val.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end_of_day = date_val.and_hms_opt(23, 59, 59).unwrap().and_utc();
                query = query
                    .filter(takeoff_time.ge(start_of_day))
                    .filter(takeoff_time.le(end_of_day));
            }

            // Apply completion filter if provided
            if let Some(completed_val) = completed {
                if completed_val {
                    // Completed flights: has landing_time OR timed_out_at
                    query = query.filter(landing_time.is_not_null().or(timed_out_at.is_not_null()));
                } else {
                    // In-progress flights: no landing_time AND no timed_out_at
                    query = query.filter(landing_time.is_null().and(timed_out_at.is_null()));
                }
            }

            // Order by most recent first
            let flight_models = query
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load::<FlightModel>(&mut conn)?;

            let result_flights: Vec<Flight> = flight_models.into_iter().map(|f| f.into()).collect();

            Ok::<Vec<Flight>, anyhow::Error>(result_flights)
        })
        .await??;

        Ok(results)
    }

    /// Mark a flight as timed out (no beacons received for the timeout duration, currently 1 hour)
    /// Does NOT set landing fields - this is a timeout, not a landing
    /// The timeout_time should be set to the last_fix_at value
    pub async fn timeout_flight(
        &self,
        flight_id: Uuid,
        timeout_time: DateTime<Utc>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    timed_out_at.eq(Some(timeout_time)),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Timeout a flight and record the flight phase when timeout occurred
    /// This helps determine coalescing behavior when aircraft reappears
    pub async fn timeout_flight_with_phase(
        &self,
        flight_id: Uuid,
        timeout_time: DateTime<Utc>,
        phase: TimeoutPhase,
        end_location: Option<Uuid>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    timed_out_at.eq(Some(timeout_time)),
                    timeout_phase.eq(Some(phase)),
                    end_location_id.eq(end_location),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Clear the timed_out_at field for a flight (set it to NULL).
    /// This is used for flight coalescing when resuming tracking of a timed-out flight.
    pub async fn clear_timeout(&self, flight_id: Uuid) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    timed_out_at.eq(None::<DateTime<Utc>>),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Update the last_fix_at timestamp for a flight
    /// This should be called whenever a new fix is assigned to a flight
    pub async fn update_last_fix_at(
        &self,
        flight_id: Uuid,
        fix_timestamp: DateTime<Utc>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((last_fix_at.eq(fix_timestamp), updated_at.eq(Utc::now())))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Update the callsign for a flight
    /// Only updates if the provided callsign is different from the current one
    pub async fn update_callsign(
        &self,
        flight_id: Uuid,
        new_callsign: Option<String>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((callsign.eq(new_callsign), updated_at.eq(Utc::now())))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Calculate and update the bounding box for a flight based on all its fixes
    /// This should be called when a flight is completed (landed or timed out)
    /// Returns true if the flight was found and updated, false otherwise
    pub async fn calculate_and_update_bounding_box(&self, flight_id: Uuid) -> Result<bool> {
        use crate::schema::fixes::dsl as fixes_dsl;
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // First, get the flight's time range to enable partition pruning
            let flight_times = flights
                .filter(id.eq(flight_id))
                .select((created_at, last_fix_at))
                .first::<(chrono::DateTime<Utc>, chrono::DateTime<Utc>)>(&mut conn)
                .optional()?;

            let bbox_result = if let Some((start_time, end_time)) = flight_times {
                // Add 1 hour buffer to handle clock skew and ensure we get all fixes
                let start_with_buffer = start_time - chrono::Duration::hours(1);
                let end_with_buffer = end_time + chrono::Duration::hours(1);

                // Calculate bounding box from all fixes for this flight
                // Filter by received_at for partition pruning (flights are always < 24h)
                fixes_dsl::fixes
                    .filter(fixes_dsl::flight_id.eq(flight_id))
                    .filter(fixes_dsl::received_at.between(start_with_buffer, end_with_buffer))
                    .select((
                        diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Double>>(
                            "MIN(latitude)",
                        ),
                        diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Double>>(
                            "MAX(latitude)",
                        ),
                        diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Double>>(
                            "MIN(longitude)",
                        ),
                        diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Double>>(
                            "MAX(longitude)",
                        ),
                    ))
                    .first::<(Option<f64>, Option<f64>, Option<f64>, Option<f64>)>(&mut conn)
                    .optional()?
            } else {
                // Flight not found, return None
                None
            };

            // If we got bounding box values, update the flight
            if let Some((min_lat, max_lat, min_lon, max_lon)) = bbox_result {
                let rows = diesel::update(flights.filter(id.eq(flight_id)))
                    .set((
                        min_latitude.eq(min_lat),
                        max_latitude.eq(max_lat),
                        min_longitude.eq(min_lon),
                        max_longitude.eq(max_lon),
                        updated_at.eq(Utc::now()),
                    ))
                    .execute(&mut conn)?;

                Ok::<usize, anyhow::Error>(rows)
            } else {
                // No fixes found for this flight, don't update
                Ok(0)
            }
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Get nearby flights that occurred within the same time frame and bounding box as a given flight
    /// Returns flights without fixes (lightweight response)
    /// Uses pre-computed bounding box columns for performance (100x faster than joining to fixes table)
    /// Only returns completed/timed out flights (active flights don't have bounding boxes yet)
    pub async fn get_nearby_flights(&self, flight_id: Uuid) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Get the target flight's bounding box and time range from flights table
            // Fall back to calculating from fixes if bounding box is not yet populated
            // Use received_at filter for partition pruning (flights are always < 24h)
            let bbox_sql = r#"
                SELECT
                    COALESCE(f.min_latitude, (
                        SELECT MIN(latitude) FROM fixes
                        WHERE flight_id = $1
                        AND received_at BETWEEN f.created_at - INTERVAL '1 hour'
                                            AND f.last_fix_at + INTERVAL '1 hour'
                    )) as min_lat,
                    COALESCE(f.max_latitude, (
                        SELECT MAX(latitude) FROM fixes
                        WHERE flight_id = $1
                        AND received_at BETWEEN f.created_at - INTERVAL '1 hour'
                                            AND f.last_fix_at + INTERVAL '1 hour'
                    )) as max_lat,
                    COALESCE(f.min_longitude, (
                        SELECT MIN(longitude) FROM fixes
                        WHERE flight_id = $1
                        AND received_at BETWEEN f.created_at - INTERVAL '1 hour'
                                            AND f.last_fix_at + INTERVAL '1 hour'
                    )) as min_lon,
                    COALESCE(f.max_longitude, (
                        SELECT MAX(longitude) FROM fixes
                        WHERE flight_id = $1
                        AND received_at BETWEEN f.created_at - INTERVAL '1 hour'
                                            AND f.last_fix_at + INTERVAL '1 hour'
                    )) as max_lon,
                    COALESCE(f.takeoff_time, f.created_at) as start_time,
                    COALESCE(f.landing_time, f.last_fix_at) as end_time
                FROM flights f
                WHERE f.id = $1
            "#;

            #[derive(QueryableByName)]
            struct FlightBounds {
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
                min_lat: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
                max_lat: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
                min_lon: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
                max_lon: Option<f64>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                start_time: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                end_time: DateTime<Utc>,
            }

            let bounds: Option<FlightBounds> = diesel::sql_query(bbox_sql)
                .bind::<diesel::sql_types::Uuid, _>(flight_id)
                .get_result(&mut conn)
                .optional()?;

            // If we can't get bounds, return empty vector
            let bounds = match bounds {
                Some(b)
                    if b.min_lat.is_some()
                        && b.max_lat.is_some()
                        && b.min_lon.is_some()
                        && b.max_lon.is_some() =>
                {
                    b
                }
                _ => return Ok(Vec::new()),
            };

            let (min_lat_val, max_lat_val, min_lon_val, max_lon_val) = (
                bounds.min_lat.unwrap(),
                bounds.max_lat.unwrap(),
                bounds.min_lon.unwrap(),
                bounds.max_lon.unwrap(),
            );

            // Query for nearby flights using bounding box columns (much faster - no JOIN to fixes!)
            // Only query flights with bounding boxes (completed/timed out flights)
            // Limit to 50 results for UI performance
            let nearby_flight_ids: Vec<Uuid> = diesel::sql_query(
                r#"
                SELECT id
                FROM flights
                WHERE id != $1
                  AND min_latitude IS NOT NULL
                  AND COALESCE(takeoff_time, created_at) <= $2
                  AND COALESCE(landing_time, last_fix_at) >= $3
                  AND min_latitude <= $5
                  AND max_latitude >= $4
                  AND min_longitude <= $7
                  AND max_longitude >= $6
                ORDER BY takeoff_time DESC
                LIMIT 50
                "#,
            )
            .bind::<diesel::sql_types::Uuid, _>(flight_id)
            .bind::<diesel::sql_types::Timestamptz, _>(bounds.end_time)
            .bind::<diesel::sql_types::Timestamptz, _>(bounds.start_time)
            .bind::<diesel::sql_types::Double, _>(min_lat_val)
            .bind::<diesel::sql_types::Double, _>(max_lat_val)
            .bind::<diesel::sql_types::Double, _>(min_lon_val)
            .bind::<diesel::sql_types::Double, _>(max_lon_val)
            .load::<OnlyId>(&mut conn)?
            .into_iter()
            .map(|r| r.id)
            .collect();

            if nearby_flight_ids.is_empty() {
                return Ok(Vec::new());
            }

            // Now get the full flight data using the type-safe query builder
            let flight_models: Vec<FlightModel> = flights
                .filter(id.eq_any(&nearby_flight_ids))
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            let result_flights: Vec<Flight> = flight_models.into_iter().map(|f| f.into()).collect();

            Ok::<Vec<Flight>, anyhow::Error>(result_flights)
        })
        .await??;

        Ok(results)
    }

    /// Get both the previous and next flights for the same device in a single database query
    /// Returns (previous_flight_id, next_flight_id) tuple
    /// More efficient than calling get_previous_flight_for_device and get_next_flight_for_device separately
    pub async fn get_adjacent_flights_for_device(
        &self,
        flight_id: Uuid,
        device_id_val: Uuid,
        current_takeoff_time: Option<DateTime<Utc>>,
    ) -> Result<(Option<Uuid>, Option<Uuid>)> {
        use diesel::sql_types::{Timestamptz, Uuid as UuidType};

        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // If there's no takeoff time, use current time as fallback
            let reference_time = current_takeoff_time.unwrap_or_else(chrono::Utc::now);

            // Use a single query with UNION ALL to get both previous and next flights
            // This is more efficient than two separate queries
            let query = r#"
                SELECT 'prev' as direction, id
                FROM flights
                WHERE aircraft_id = $1
                  AND id != $2
                  AND (
                    takeoff_time < $3
                    OR (takeoff_time IS NULL AND created_at < $3)
                  )
                ORDER BY COALESCE(takeoff_time, created_at) DESC
                LIMIT 1

                UNION ALL

                SELECT 'next' as direction, id
                FROM flights
                WHERE aircraft_id = $1
                  AND id != $2
                  AND (
                    takeoff_time > $3
                    OR (takeoff_time IS NULL AND created_at > $3)
                  )
                ORDER BY COALESCE(takeoff_time, created_at) ASC
                LIMIT 1
            "#;

            #[derive(QueryableByName)]
            struct AdjacentFlight {
                #[diesel(sql_type = diesel::sql_types::Text)]
                direction: String,
                #[diesel(sql_type = UuidType)]
                id: Uuid,
            }

            let results: Vec<AdjacentFlight> = diesel::sql_query(query)
                .bind::<UuidType, _>(device_id_val)
                .bind::<UuidType, _>(flight_id)
                .bind::<Timestamptz, _>(reference_time)
                .load(&mut conn)?;

            let mut prev_flight = None;
            let mut next_flight = None;

            for result in results {
                match result.direction.as_str() {
                    "prev" => prev_flight = Some(result.id),
                    "next" => next_flight = Some(result.id),
                    _ => {}
                }
            }

            Ok::<(Option<Uuid>, Option<Uuid>), anyhow::Error>((prev_flight, next_flight))
        })
        .await??;

        Ok(result)
    }

    /// Get the previous flight for the same device (chronologically earlier by takeoff time)
    /// Returns None if there is no previous flight
    /// Note: For better performance when getting both previous and next, use get_adjacent_flights_for_device
    pub async fn get_previous_flight_for_device(
        &self,
        flight_id: Uuid,
        device_id_val: Uuid,
        current_takeoff_time: Option<DateTime<Utc>>,
    ) -> Result<Option<Uuid>> {
        let (prev, _) = self
            .get_adjacent_flights_for_device(flight_id, device_id_val, current_takeoff_time)
            .await?;
        Ok(prev)
    }

    /// Get the next flight for the same device (chronologically later by takeoff time)
    /// Returns None if there is no next flight
    /// Note: For better performance when getting both previous and next, use get_adjacent_flights_for_device
    pub async fn get_next_flight_for_device(
        &self,
        flight_id: Uuid,
        device_id_val: Uuid,
        current_takeoff_time: Option<DateTime<Utc>>,
    ) -> Result<Option<Uuid>> {
        let (_, next) = self
            .get_adjacent_flights_for_device(flight_id, device_id_val, current_takeoff_time)
            .await?;
        Ok(next)
    }

    /// Timeout all incomplete flights where last_fix_at is older than the specified duration
    /// Sets timed_out_at to last_fix_at + timeout_duration for these flights
    /// Returns the number of flights that were timed out
    pub async fn timeout_old_incomplete_flights(
        &self,
        timeout_duration: chrono::Duration,
    ) -> Result<usize> {
        use diesel::sql_types::Timestamptz;

        let pool = self.pool.clone();
        let cutoff_time = Utc::now() - timeout_duration;

        // Convert chrono::Duration to PostgreSQL interval string
        // PostgreSQL interval format: 'X hours' or 'X seconds'
        let interval_str = format!("{} seconds", timeout_duration.num_seconds());

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL to perform the update with PostgreSQL interval arithmetic
            // UPDATE flights
            // SET timed_out_at = last_fix_at + INTERVAL 'X seconds',
            //     updated_at = NOW()
            // WHERE timed_out_at IS NULL
            //   AND landing_time IS NULL
            //   AND last_fix_at < $1
            let query = format!(
                "UPDATE flights \
                 SET timed_out_at = last_fix_at + INTERVAL '{}', \
                     updated_at = NOW() \
                 WHERE timed_out_at IS NULL \
                   AND landing_time IS NULL \
                   AND last_fix_at < $1",
                interval_str
            );

            let rows = diesel::sql_query(query)
                .bind::<Timestamptz, _>(cutoff_time)
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected)
    }

    /// Get all active flights (no landing_time, no timed_out_at, last_fix_at within timeout_duration)
    /// Returns flights that should be loaded into the flight tracker on startup
    pub async fn get_active_flights_for_tracker(
        &self,
        timeout_duration: chrono::Duration,
    ) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();
        let cutoff_time = Utc::now() - timeout_duration;

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Get flights where:
            // - timed_out_at is NULL (not timed out)
            // - landing_time is NULL (not landed)
            // - last_fix_at is within the timeout window (recent enough to be active)
            let flight_models: Vec<FlightModel> = flights
                .filter(timed_out_at.is_null())
                .filter(landing_time.is_null())
                .filter(last_fix_at.ge(cutoff_time))
                .order(last_fix_at.desc())
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            let result_flights: Vec<Flight> = flight_models.into_iter().map(|f| f.into()).collect();

            Ok::<Vec<Flight>, anyhow::Error>(result_flights)
        })
        .await??;

        Ok(results)
    }
}

#[derive(QueryableByName)]
struct OnlyId {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}
