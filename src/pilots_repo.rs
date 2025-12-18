use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

use crate::pilots::{FlightPilot, FlightPilotModel};
use crate::users::User;
use crate::users_repo::UserRecord;

type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Repository for managing flight-pilot links (junction table between flights and users)
pub struct PilotsRepository {
    pool: PgPool,
}

impl PilotsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Link a pilot (user) to a flight with specific roles
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

    /// Get all pilots (users) for a specific flight
    /// Returns tuples of (User, FlightPilot) with the user info and their role in the flight
    pub async fn get_pilots_for_flight(
        &self,
        flight_id_val: Uuid,
    ) -> Result<Vec<(User, FlightPilot)>> {
        use crate::schema::flight_pilots;
        use crate::schema::users;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let results: Vec<(UserRecord, FlightPilotModel)> = users::table
                .inner_join(flight_pilots::table.on(users::id.eq(flight_pilots::user_id)))
                .filter(flight_pilots::flight_id.eq(flight_id_val))
                .select((UserRecord::as_select(), FlightPilotModel::as_select()))
                .load(&mut conn)?;

            Ok::<Vec<(UserRecord, FlightPilotModel)>, anyhow::Error>(results)
        })
        .await??;

        Ok(results
            .into_iter()
            .map(|(user_record, flight_pilot_model)| {
                (user_record.into(), flight_pilot_model.into())
            })
            .collect())
    }

    /// Get all flights for a specific pilot (user)
    pub async fn get_flights_for_pilot(&self, user_id_val: Uuid) -> Result<Vec<FlightPilot>> {
        use crate::schema::flight_pilots::dsl::*;

        let pool = self.pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let flight_pilot_models: Vec<FlightPilotModel> = flight_pilots
                .filter(user_id.eq(user_id_val))
                .order(created_at.desc())
                .select(FlightPilotModel::as_select())
                .load(&mut conn)?;

            Ok::<Vec<FlightPilotModel>, anyhow::Error>(flight_pilot_models)
        })
        .await??;

        Ok(results.into_iter().map(|model| model.into()).collect())
    }

    /// Remove a pilot (user) from a flight
    pub async fn unlink_pilot_from_flight(
        &self,
        flight_id_val: Uuid,
        user_id_val: Uuid,
    ) -> Result<bool> {
        use crate::schema::flight_pilots::dsl::*;

        let pool = self.pool.clone();

        let rows_affected = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let rows = diesel::delete(
                flight_pilots
                    .filter(flight_id.eq(flight_id_val))
                    .filter(user_id.eq(user_id_val)),
            )
            .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows_affected > 0)
    }
}
