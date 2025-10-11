use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::flights::{Flight, FlightModel};
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

    /// Update landing time for a flight
    pub async fn update_landing_time(
        &self,
        flight_id: Uuid,
        landing_time_param: DateTime<Utc>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    landing_time.eq(&Some(landing_time_param)),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Update flight with landing information
    #[allow(clippy::too_many_arguments)]
    pub async fn update_flight_landing(
        &self,
        flight_id: Uuid,
        landing_time_param: DateTime<Utc>,
        arrival_airport_id_param: Option<i32>,
        landing_location_id_param: Option<Uuid>,
        landing_altitude_offset_ft_param: Option<i32>,
        landing_runway_ident_param: Option<String>,
        total_distance_meters_param: Option<f64>,
        maximum_displacement_meters_param: Option<f64>,
        runways_inferred_param: Option<bool>,
    ) -> Result<()> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    landing_time.eq(&Some(landing_time_param)),
                    arrival_airport_id.eq(&arrival_airport_id_param),
                    landing_location_id.eq(&landing_location_id_param),
                    landing_altitude_offset_ft.eq(&landing_altitude_offset_ft_param),
                    landing_runway_ident.eq(&landing_runway_ident_param),
                    total_distance_meters.eq(&total_distance_meters_param),
                    maximum_displacement_meters.eq(&maximum_displacement_meters_param),
                    runways_inferred.eq(&runways_inferred_param),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

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
                .filter(device_id.eq(device_id_val))
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
                .filter(device_id.eq(device_id_val))
                .count()
                .get_result::<i64>(&mut conn)?;

            // Get paginated results
            let offset = (page - 1) * per_page;
            let flight_models: Vec<FlightModel> = flights
                .filter(device_id.eq(device_id_val))
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

    /// Get all flights in progress (no landing time) ordered by takeoff time descending
    pub async fn get_flights_in_progress(&self) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(landing_time.is_null())
                .order(takeoff_time.desc())
                .select(FlightModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightModel>, anyhow::Error>(flight_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Get recent completed flights (with landing time) ordered by landing time descending
    pub async fn get_completed_flights(&self, limit: i64) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(landing_time.is_not_null())
                .order(landing_time.desc())
                .limit(limit)
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

    /// Get flights that used a specific tow aircraft
    pub async fn get_flights_by_tow_aircraft(
        &self,
        tow_aircraft_id_param: &str,
    ) -> Result<Vec<Flight>> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();
        let tow_aircraft_id_val = tow_aircraft_id_param.to_string();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_models: Vec<FlightModel> = flights
                .filter(tow_aircraft_id.eq(&Some(tow_aircraft_id_val)))
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

    /// Get the count of flights in progress
    pub async fn get_flights_in_progress_count(&self) -> Result<i64> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let count = flights
                .filter(landing_time.is_null())
                .count()
                .get_result::<i64>(&mut conn)?;

            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(count)
    }

    /// Update flight details (departure/arrival airports, tow info)
    pub async fn update_flight_details(
        &self,
        flight_id: Uuid,
        departure_airport_id_param: Option<i32>,
        arrival_airport_id_param: Option<i32>,
        tow_aircraft_id_param: Option<String>,
        tow_release_height_msl_param: Option<i32>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    departure_airport_id.eq(&departure_airport_id_param),
                    arrival_airport_id.eq(&arrival_airport_id_param),
                    tow_aircraft_id.eq(&tow_aircraft_id_param),
                    tow_release_height_msl.eq(&tow_release_height_msl_param),
                    updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
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
                    towed_by_device_id.eq(Some(towplane_device_id)),
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
    pub async fn update_tow_release(
        &self,
        glider_flight_id: Uuid,
        release_altitude_ft: i32,
        release_time: DateTime<Utc>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(glider_flight_id)))
                .set((
                    tow_release_altitude_msl_ft.eq(Some(release_altitude_ft)),
                    tow_release_time.eq(Some(release_time)),
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

    /// Get flights for a specific date (all flights that took off on that date)
    pub async fn get_flights_for_date(&self, date: chrono::NaiveDate) -> Result<Vec<Flight>> {
        let start_of_day = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_of_day = date.and_hms_opt(23, 59, 59).unwrap().and_utc();

        self.get_flights_in_time_range(start_of_day, end_of_day, None)
            .await
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
                .load::<FlightModel>(&mut conn)?;

            let result_flights: Vec<Flight> = flight_models.into_iter().map(|f| f.into()).collect();

            Ok::<Vec<Flight>, anyhow::Error>(result_flights)
        })
        .await??;

        Ok(results)
    }

    /// Mark a flight as timed out (no beacons received for 5+ minutes)
    /// Does NOT set landing fields - this is a timeout, not a landing
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
}
