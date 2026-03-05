use anyhow::Result;
use diesel::prelude::*;
use uuid::Uuid;

use crate::club_join_requests::{
    ClubJoinRequest, ClubJoinRequestModel, NewClubJoinRequest, STATUS_APPROVED, STATUS_PENDING,
    STATUS_REJECTED,
};
use crate::web::PgPool;

#[derive(Clone)]
pub struct ClubJoinRequestsRepository {
    pool: PgPool,
}

impl ClubJoinRequestsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all pending join requests for a club, including requester names
    pub async fn get_pending_by_club(&self, club_id: Uuid) -> Result<Vec<ClubJoinRequest>> {
        use crate::schema::club_join_requests::dsl;
        use crate::schema::users;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let requests: Vec<(ClubJoinRequestModel, String, String)> = dsl::club_join_requests
                .inner_join(users::table.on(users::id.eq(dsl::user_id)))
                .filter(dsl::club_id.eq(club_id))
                .filter(dsl::status.eq(STATUS_PENDING))
                .order_by(dsl::created_at.asc())
                .select((
                    ClubJoinRequestModel::as_select(),
                    users::first_name,
                    users::last_name,
                ))
                .load(&mut conn)?;

            Ok::<Vec<(ClubJoinRequestModel, String, String)>, anyhow::Error>(requests)
        })
        .await??;

        Ok(result
            .into_iter()
            .map(|(m, first_name, last_name)| {
                let mut req: ClubJoinRequest = m.into();
                req.user_first_name = Some(first_name);
                req.user_last_name = Some(last_name);
                req
            })
            .collect())
    }

    /// Get a user's pending request for a specific club
    pub async fn get_pending_by_user_and_club(
        &self,
        user_id: Uuid,
        club_id: Uuid,
    ) -> Result<Option<ClubJoinRequest>> {
        use crate::schema::club_join_requests::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let request_opt: Option<ClubJoinRequestModel> = dsl::club_join_requests
                .filter(dsl::user_id.eq(user_id))
                .filter(dsl::club_id.eq(club_id))
                .filter(dsl::status.eq(STATUS_PENDING))
                .first::<ClubJoinRequestModel>(&mut conn)
                .optional()?;

            Ok::<Option<ClubJoinRequestModel>, anyhow::Error>(request_opt)
        })
        .await??;

        Ok(result.map(|m| m.into()))
    }

    /// Get a specific request by ID
    pub async fn get_by_id(&self, request_id: Uuid) -> Result<Option<ClubJoinRequest>> {
        use crate::schema::club_join_requests::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let request_opt: Option<ClubJoinRequestModel> = dsl::club_join_requests
                .filter(dsl::id.eq(request_id))
                .first::<ClubJoinRequestModel>(&mut conn)
                .optional()?;

            Ok::<Option<ClubJoinRequestModel>, anyhow::Error>(request_opt)
        })
        .await??;

        Ok(result.map(|m| m.into()))
    }

    /// Create a new join request
    pub async fn create(&self, new_request: NewClubJoinRequest) -> Result<ClubJoinRequest> {
        use crate::schema::club_join_requests::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let inserted: ClubJoinRequestModel = diesel::insert_into(dsl::club_join_requests)
                .values(&new_request)
                .get_result(&mut conn)?;

            Ok::<ClubJoinRequestModel, anyhow::Error>(inserted)
        })
        .await??;

        Ok(result.into())
    }

    /// Reject a join request
    pub async fn reject(
        &self,
        request_id: Uuid,
        reviewed_by: Uuid,
    ) -> Result<Option<ClubJoinRequest>> {
        use crate::schema::club_join_requests;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let updated: Option<ClubJoinRequestModel> = diesel::update(club_join_requests::table)
                .filter(club_join_requests::id.eq(request_id))
                .filter(club_join_requests::status.eq(STATUS_PENDING))
                .set((
                    club_join_requests::status.eq(STATUS_REJECTED),
                    club_join_requests::reviewed_by.eq(reviewed_by),
                    club_join_requests::reviewed_at.eq(diesel::dsl::now),
                    club_join_requests::updated_at.eq(diesel::dsl::now),
                ))
                .get_result(&mut conn)
                .optional()?;

            Ok::<Option<ClubJoinRequestModel>, anyhow::Error>(updated)
        })
        .await??;

        Ok(result.map(|m| m.into()))
    }

    /// Approve a join request and update the user's club_id atomically.
    /// Also checks that the user is not already in a different club (TOCTOU-safe).
    pub async fn approve_and_set_club(
        &self,
        request_id: Uuid,
        reviewed_by: Uuid,
    ) -> Result<Option<ClubJoinRequest>> {
        use crate::schema::{club_join_requests, users};

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(anyhow::Error::from)?;

            let updated = conn.transaction(|conn| {
                // Approve the request
                let updated: Option<ClubJoinRequestModel> =
                    diesel::update(club_join_requests::table)
                        .filter(club_join_requests::id.eq(request_id))
                        .filter(club_join_requests::status.eq(STATUS_PENDING))
                        .set((
                            club_join_requests::status.eq(STATUS_APPROVED),
                            club_join_requests::reviewed_by.eq(reviewed_by),
                            club_join_requests::reviewed_at.eq(diesel::dsl::now),
                            club_join_requests::updated_at.eq(diesel::dsl::now),
                        ))
                        .get_result(conn)
                        .optional()?;

                if let Some(ref approved) = updated {
                    // Check user's current club_id inside the transaction to prevent TOCTOU races
                    let current_club_id: Option<uuid::Uuid> = users::table
                        .filter(users::id.eq(approved.user_id))
                        .select(users::club_id)
                        .first(conn)?;

                    if current_club_id.is_some_and(|cid| cid != approved.club_id) {
                        return Err(diesel::result::Error::RollbackTransaction);
                    }

                    // Update the user's club_id in the same transaction
                    diesel::update(users::table)
                        .filter(users::id.eq(approved.user_id))
                        .set(users::club_id.eq(approved.club_id))
                        .execute(conn)?;
                }

                Ok::<Option<ClubJoinRequestModel>, diesel::result::Error>(updated)
            });

            match updated {
                Ok(result) => Ok(result),
                Err(diesel::result::Error::RollbackTransaction) => {
                    Err(anyhow::anyhow!("User is already a member of another club"))
                }
                Err(e) => Err(anyhow::Error::from(e)),
            }
        })
        .await??;

        Ok(result.map(|m| m.into()))
    }

    /// Cancel (delete) a pending join request
    pub async fn cancel(&self, request_id: Uuid) -> Result<bool> {
        use crate::schema::club_join_requests::dsl;

        let pool = self.pool.clone();
        let deleted_count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let count = diesel::delete(dsl::club_join_requests)
                .filter(dsl::id.eq(request_id))
                .filter(dsl::status.eq(STATUS_PENDING))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(count)
        })
        .await??;

        Ok(deleted_count > 0)
    }
}
