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
    pub async fn update_flight_landing(
        &self,
        flight_id: Uuid,
        landing_time_param: DateTime<Utc>,
        arrival_airport_param: Option<String>,
    ) -> Result<()> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    landing_time.eq(&Some(landing_time_param)),
                    arrival_airport.eq(&arrival_airport_param),
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
        departure_airport_param: Option<String>,
        arrival_airport_param: Option<String>,
        tow_aircraft_id_param: Option<String>,
        tow_release_height_msl_param: Option<i32>,
    ) -> Result<bool> {
        use crate::schema::flights::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(flights.filter(id.eq(flight_id)))
                .set((
                    departure_airport.eq(&departure_airport_param),
                    arrival_airport.eq(&arrival_airport_param),
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
}
