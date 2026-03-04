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

    /// Get all pending join requests for a club
    pub async fn get_pending_by_club(&self, club_id: Uuid) -> Result<Vec<ClubJoinRequest>> {
        use crate::schema::club_join_requests::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let requests: Vec<ClubJoinRequestModel> = dsl::club_join_requests
                .filter(dsl::club_id.eq(club_id))
                .filter(dsl::status.eq(STATUS_PENDING))
                .order_by(dsl::created_at.asc())
                .load::<ClubJoinRequestModel>(&mut conn)?;

            Ok::<Vec<ClubJoinRequestModel>, anyhow::Error>(requests)
        })
        .await??;

        Ok(result.into_iter().map(|m| m.into()).collect())
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

    /// Approve a join request (sets status and reviewer)
    pub async fn approve(
        &self,
        request_id: Uuid,
        reviewed_by: Uuid,
    ) -> Result<Option<ClubJoinRequest>> {
        use crate::schema::club_join_requests;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

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
                    .get_result(&mut conn)
                    .optional()?;

            Ok::<Option<ClubJoinRequestModel>, anyhow::Error>(updated)
        })
        .await??;

        Ok(result.map(|m| m.into()))
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

            let updated: Option<ClubJoinRequestModel> =
                diesel::update(club_join_requests::table)
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
