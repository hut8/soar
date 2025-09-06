use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::flights::Flight;

pub struct FlightsRepository {
    pool: PgPool,
}

impl FlightsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new flight into the database
    pub async fn insert_flight(&self, flight: &Flight) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO flights (
                id, aircraft_id, takeoff_time, landing_time, departure_airport,
                arrival_airport, tow_aircraft_id, tow_release_height_msl,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            flight.id,
            flight.aircraft_id,
            flight.takeoff_time,
            flight.landing_time,
            flight.departure_airport,
            flight.arrival_airport,
            flight.tow_aircraft_id,
            flight.tow_release_height_msl,
            flight.created_at,
            flight.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update landing time for a flight
    pub async fn update_landing_time(
        &self,
        flight_id: Uuid,
        landing_time: DateTime<Utc>,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE flights 
            SET landing_time = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            landing_time,
            flight_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get a flight by its ID
    pub async fn get_flight_by_id(&self, flight_id: Uuid) -> Result<Option<Flight>> {
        let result = sqlx::query!(
            r#"
            SELECT id, aircraft_id, takeoff_time, landing_time, departure_airport,
                   arrival_airport, tow_aircraft_id, tow_release_height_msl,
                   created_at, updated_at
            FROM flights
            WHERE id = $1
            "#,
            flight_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            Ok(Some(Flight {
                id: row.id,
                aircraft_id: row.aircraft_id,
                takeoff_time: row.takeoff_time,
                landing_time: row.landing_time,
                departure_airport: row.departure_airport,
                arrival_airport: row.arrival_airport,
                tow_aircraft_id: row.tow_aircraft_id,
                tow_release_height_msl: row.tow_release_height_msl,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all flights for a specific aircraft, ordered by takeoff time descending
    pub async fn get_flights_for_aircraft(&self, aircraft_id: &str) -> Result<Vec<Flight>> {
        let results = sqlx::query!(
            r#"
            SELECT id, aircraft_id, takeoff_time, landing_time, departure_airport,
                   arrival_airport, tow_aircraft_id, tow_release_height_msl,
                   created_at, updated_at
            FROM flights
            WHERE aircraft_id = $1
            ORDER BY takeoff_time DESC
            "#,
            aircraft_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut flights = Vec::new();
        for row in results {
            flights.push(Flight {
                id: row.id,
                aircraft_id: row.aircraft_id,
                takeoff_time: row.takeoff_time,
                landing_time: row.landing_time,
                departure_airport: row.departure_airport,
                arrival_airport: row.arrival_airport,
                tow_aircraft_id: row.tow_aircraft_id,
                tow_release_height_msl: row.tow_release_height_msl,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(flights)
    }

    /// Get all flights in progress (no landing time) ordered by takeoff time descending
    pub async fn get_flights_in_progress(&self) -> Result<Vec<Flight>> {
        let results = sqlx::query!(
            r#"
            SELECT id, aircraft_id, takeoff_time, landing_time, departure_airport,
                   arrival_airport, tow_aircraft_id, tow_release_height_msl,
                   created_at, updated_at
            FROM flights
            WHERE landing_time IS NULL
            ORDER BY takeoff_time DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut flights = Vec::new();
        for row in results {
            flights.push(Flight {
                id: row.id,
                aircraft_id: row.aircraft_id,
                takeoff_time: row.takeoff_time,
                landing_time: row.landing_time,
                departure_airport: row.departure_airport,
                arrival_airport: row.arrival_airport,
                tow_aircraft_id: row.tow_aircraft_id,
                tow_release_height_msl: row.tow_release_height_msl,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(flights)
    }

    /// Get flights within a time range, optionally filtered by aircraft
    pub async fn get_flights_in_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        aircraft_id: Option<&str>,
    ) -> Result<Vec<Flight>> {
        let results = if let Some(aircraft_id) = aircraft_id {
            sqlx::query!(
                r#"
                SELECT id, aircraft_id, takeoff_time, landing_time, departure_airport,
                       arrival_airport, tow_aircraft_id, tow_release_height_msl,
                       created_at, updated_at
                FROM flights
                WHERE aircraft_id = $1 
                AND takeoff_time >= $2 
                AND takeoff_time <= $3
                ORDER BY takeoff_time DESC
                "#,
                aircraft_id,
                start_time,
                end_time
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query!(
                r#"
                SELECT id, aircraft_id, takeoff_time, landing_time, departure_airport,
                       arrival_airport, tow_aircraft_id, tow_release_height_msl,
                       created_at, updated_at
                FROM flights
                WHERE takeoff_time >= $1 
                AND takeoff_time <= $2
                ORDER BY takeoff_time DESC
                "#,
                start_time,
                end_time
            )
            .fetch_all(&self.pool)
            .await?
        };

        let mut flights = Vec::new();
        for row in results {
            flights.push(Flight {
                id: row.id,
                aircraft_id: row.aircraft_id,
                takeoff_time: row.takeoff_time,
                landing_time: row.landing_time,
                departure_airport: row.departure_airport,
                arrival_airport: row.arrival_airport,
                tow_aircraft_id: row.tow_aircraft_id,
                tow_release_height_msl: row.tow_release_height_msl,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(flights)
    }

    /// Get flights that used a specific tow aircraft
    pub async fn get_flights_by_tow_aircraft(&self, tow_aircraft_id: &str) -> Result<Vec<Flight>> {
        let results = sqlx::query!(
            r#"
            SELECT id, aircraft_id, takeoff_time, landing_time, departure_airport,
                   arrival_airport, tow_aircraft_id, tow_release_height_msl,
                   created_at, updated_at
            FROM flights
            WHERE tow_aircraft_id = $1
            ORDER BY takeoff_time DESC
            "#,
            tow_aircraft_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut flights = Vec::new();
        for row in results {
            flights.push(Flight {
                id: row.id,
                aircraft_id: row.aircraft_id,
                takeoff_time: row.takeoff_time,
                landing_time: row.landing_time,
                departure_airport: row.departure_airport,
                arrival_airport: row.arrival_airport,
                tow_aircraft_id: row.tow_aircraft_id,
                tow_release_height_msl: row.tow_release_height_msl,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(flights)
    }

    /// Get the total count of flights in the database
    pub async fn get_flight_count(&self) -> Result<i64> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM flights")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Get the count of flights in progress
    pub async fn get_flights_in_progress_count(&self) -> Result<i64> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM flights WHERE landing_time IS NULL"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.count.unwrap_or(0))
    }

    /// Update flight details (departure/arrival airports, tow info)
    pub async fn update_flight_details(
        &self,
        flight_id: Uuid,
        departure_airport: Option<String>,
        arrival_airport: Option<String>,
        tow_aircraft_id: Option<String>,
        tow_release_height_msl: Option<i32>,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE flights 
            SET departure_airport = $1, 
                arrival_airport = $2, 
                tow_aircraft_id = $3, 
                tow_release_height_msl = $4,
                updated_at = NOW()
            WHERE id = $5
            "#,
            departure_airport,
            arrival_airport,
            tow_aircraft_id,
            tow_release_height_msl,
            flight_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a flight by ID
    pub async fn delete_flight(&self, flight_id: Uuid) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM flights WHERE id = $1", flight_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get flights for a specific date (all flights that took off on that date)
    pub async fn get_flights_for_date(&self, date: chrono::NaiveDate) -> Result<Vec<Flight>> {
        let start_of_day = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_of_day = date.and_hms_opt(23, 59, 59).unwrap().and_utc();

        self.get_flights_in_time_range(start_of_day, end_of_day, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_flight() -> Flight {
        Flight::new("39D304".to_string(), Utc::now())
    }

    #[test]
    fn test_flight_creation() {
        let flight = create_test_flight();
        assert_eq!(flight.aircraft_id, "39D304");
        assert!(flight.is_in_progress());
        assert!(flight.duration().num_seconds() >= 0);
    }
}