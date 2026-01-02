use anyhow::Result;
use diesel::prelude::*;
use uuid::Uuid;

use crate::club_tow_fees::{ClubTowFee, ClubTowFeeModel, NewClubTowFee, UpdateClubTowFee};
use crate::web::PgPool;

#[derive(Clone)]
pub struct ClubTowFeesRepository {
    pool: PgPool,
}

impl ClubTowFeesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all tow fees for a specific club, ordered by altitude (NULL last)
    pub async fn get_by_club_id(&self, club_id: Uuid) -> Result<Vec<ClubTowFee>> {
        use crate::schema::club_tow_fees::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let fees: Vec<ClubTowFeeModel> = dsl::club_tow_fees
                .filter(dsl::club_id.eq(club_id))
                .order_by((dsl::max_altitude.asc().nulls_last(), dsl::created_at.asc()))
                .load::<ClubTowFeeModel>(&mut conn)?;

            Ok::<Vec<ClubTowFeeModel>, anyhow::Error>(fees)
        })
        .await??;

        Ok(result.into_iter().map(|model| model.into()).collect())
    }

    /// Get a specific tow fee by ID
    pub async fn get_by_id(&self, fee_id: Uuid) -> Result<Option<ClubTowFee>> {
        use crate::schema::club_tow_fees::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let fee_opt: Option<ClubTowFeeModel> = dsl::club_tow_fees
                .filter(dsl::id.eq(fee_id))
                .first::<ClubTowFeeModel>(&mut conn)
                .optional()?;

            Ok::<Option<ClubTowFeeModel>, anyhow::Error>(fee_opt)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }

    /// Create a new tow fee tier
    pub async fn create(&self, new_fee: NewClubTowFee) -> Result<ClubTowFee> {
        use crate::schema::club_tow_fees::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let inserted_fee: ClubTowFeeModel = diesel::insert_into(dsl::club_tow_fees)
                .values(&new_fee)
                .get_result(&mut conn)?;

            Ok::<ClubTowFeeModel, anyhow::Error>(inserted_fee)
        })
        .await??;

        Ok(result.into())
    }

    /// Update an existing tow fee tier
    pub async fn update(&self, fee_id: Uuid, update_fee: UpdateClubTowFee) -> Result<ClubTowFee> {
        use crate::schema::club_tow_fees;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let updated_fee: ClubTowFeeModel = diesel::update(club_tow_fees::table)
                .filter(club_tow_fees::id.eq(fee_id))
                .set((
                    club_tow_fees::max_altitude.eq(update_fee.max_altitude),
                    club_tow_fees::cost.eq(update_fee.cost),
                    club_tow_fees::modified_by.eq(update_fee.modified_by),
                    club_tow_fees::updated_at.eq(diesel::dsl::now),
                ))
                .get_result(&mut conn)?;

            Ok::<ClubTowFeeModel, anyhow::Error>(updated_fee)
        })
        .await??;

        Ok(result.into())
    }

    /// Delete a tow fee tier
    pub async fn delete(&self, fee_id: Uuid) -> Result<bool> {
        use crate::schema::club_tow_fees::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let deleted_count = diesel::delete(dsl::club_tow_fees)
                .filter(dsl::id.eq(fee_id))
                .execute(&mut conn)?;

            Ok::<usize, anyhow::Error>(deleted_count)
        })
        .await??;

        Ok(result > 0)
    }

    /// Check if a club has a fallback tier (NULL max_altitude)
    pub async fn has_fallback_tier(&self, club_id: Uuid) -> Result<bool> {
        use crate::schema::club_tow_fees::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let count: i64 = dsl::club_tow_fees
                .filter(dsl::club_id.eq(club_id))
                .filter(dsl::max_altitude.is_null())
                .count()
                .get_result(&mut conn)?;

            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(result > 0)
    }

    /// Get the fallback tier for a club (NULL max_altitude)
    pub async fn get_fallback_tier(&self, club_id: Uuid) -> Result<Option<ClubTowFee>> {
        use crate::schema::club_tow_fees::dsl;

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let fee_opt: Option<ClubTowFeeModel> = dsl::club_tow_fees
                .filter(dsl::club_id.eq(club_id))
                .filter(dsl::max_altitude.is_null())
                .first::<ClubTowFeeModel>(&mut conn)
                .optional()?;

            Ok::<Option<ClubTowFeeModel>, anyhow::Error>(fee_opt)
        })
        .await??;

        Ok(result.map(|model| model.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    #[test]
    fn test_new_club_tow_fee() {
        let club_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let new_fee = NewClubTowFee {
            club_id,
            max_altitude: Some(2000),
            cost: BigDecimal::from_str("45.00").unwrap(),
            modified_by: user_id,
        };

        assert_eq!(new_fee.club_id, club_id);
        assert_eq!(new_fee.max_altitude, Some(2000));
        assert_eq!(new_fee.modified_by, user_id);
    }

    // Note: Integration tests would require a test database setup
    // These are structural examples showing expected behavior
}
