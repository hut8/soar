use anyhow::Result;
use diesel::prelude::*;
use uuid::Uuid;

use crate::pilots::{FlightPilot, FlightPilotModel, Pilot, PilotModel};
use crate::web::PgPool;

#[derive(Clone)]
pub struct PilotsRepository {
    pool: PgPool,
}

impl PilotsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new pilot and insert it into the database
    pub async fn create_pilot(&self, pilot: Pilot) -> Result<()> {
        self.insert_pilot(&pilot).await
    }

    /// Insert a new pilot into the database
    pub async fn insert_pilot(&self, pilot: &Pilot) -> Result<()> {
        use crate::schema::pilots;

        let pool = self.pool.clone();
        let pilot_model: PilotModel = pilot.clone().into();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::insert_into(pilots::table)
                .values(&pilot_model)
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    /// Get a pilot by ID
    pub async fn get_pilot_by_id(&self, pilot_id: Uuid) -> Result<Option<Pilot>> {
        use crate::schema::pilots::dsl::*;

        let pool = self.pool.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let pilot_model: Option<PilotModel> = pilots
                .filter(id.eq(pilot_id))
                .select(PilotModel::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<PilotModel>, anyhow::Error>(pilot_model)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Get all pilots for a specific club
    pub async fn get_pilots_by_club(&self, club_id_val: Uuid) -> Result<Vec<Pilot>> {
        use crate::schema::pilots::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let pilot_models: Vec<PilotModel> = pilots
                .filter(club_id.eq(Some(club_id_val)))
                .filter(deleted_at.is_null()) // Exclude soft-deleted pilots
                .order((last_name.asc(), first_name.asc()))
                .select(PilotModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<PilotModel>, anyhow::Error>(pilot_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Update a pilot
    pub async fn update_pilot(&self, pilot: &Pilot) -> Result<bool> {
        use crate::schema::pilots::dsl::*;

        let pool = self.pool.clone();
        let pilot_model: PilotModel = pilot.clone().into();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(pilots.filter(id.eq(pilot_model.id)))
                .set(&pilot_model)
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Soft delete a pilot by ID (sets deleted_at timestamp)
    pub async fn delete_pilot(&self, pilot_id: Uuid) -> Result<bool> {
        use crate::schema::pilots::dsl::*;
        use chrono::Utc;

        let pool = self.pool.clone();
        let now = Utc::now();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::update(pilots.filter(id.eq(pilot_id)))
                .set(deleted_at.eq(Some(now)))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }

    /// Get the total count of pilots in the database
    pub async fn get_pilot_count(&self) -> Result<i64> {
        use crate::schema::pilots::dsl::*;

        let pool = self.pool.clone();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let count = pilots.count().get_result::<i64>(&mut conn)?;

            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(count)
    }

    /// Link a pilot to a flight with specific roles
    pub async fn link_pilot_to_flight(&self, flight_pilot: &FlightPilot) -> Result<()> {
        use crate::schema::flight_pilots;

        let pool = self.pool.clone();
        let flight_pilot_model: FlightPilotModel = flight_pilot.clone().into();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::insert_into(flight_pilots::table)
                .values(&flight_pilot_model)
                .execute(&mut conn)?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    /// Get all pilots for a specific flight
    pub async fn get_pilots_for_flight(
        &self,
        flight_id_val: Uuid,
    ) -> Result<Vec<(Pilot, FlightPilot)>> {
        use crate::schema::flight_pilots;
        use crate::schema::pilots;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let results: Vec<(PilotModel, FlightPilotModel)> = pilots::table
                .inner_join(flight_pilots::table.on(pilots::id.eq(flight_pilots::pilot_id)))
                .filter(flight_pilots::flight_id.eq(flight_id_val))
                .select((PilotModel::as_select(), FlightPilotModel::as_select()))
                .load(&mut conn)?;

            Ok::<Vec<(PilotModel, FlightPilotModel)>, anyhow::Error>(results)
        })
        .await??;

        Ok(results
            .into_iter()
            .map(|(pilot_model, flight_pilot_model)| {
                (pilot_model.into(), flight_pilot_model.into())
            })
            .collect())
    }

    /// Get all flights for a specific pilot
    pub async fn get_flights_for_pilot(&self, pilot_id_val: Uuid) -> Result<Vec<FlightPilot>> {
        use crate::schema::flight_pilots::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_pilot_models: Vec<FlightPilotModel> = flight_pilots
                .filter(pilot_id.eq(pilot_id_val))
                .order(created_at.desc())
                .select(FlightPilotModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightPilotModel>, anyhow::Error>(flight_pilot_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Remove a pilot from a flight
    pub async fn unlink_pilot_from_flight(
        &self,
        flight_id_val: Uuid,
        pilot_id_val: Uuid,
    ) -> Result<bool> {
        use crate::schema::flight_pilots::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::delete(
                flight_pilots
                    .filter(flight_id.eq(flight_id_val))
                    .filter(pilot_id.eq(pilot_id_val)),
            )
            .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }
}
