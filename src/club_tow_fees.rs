use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API model for club tow fees - used for JSON serialization/deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubTowFee {
    pub id: Uuid,
    pub club_id: Uuid,
    /// Maximum altitude in feet AGL for this tier. None means "anything above the highest specified altitude"
    pub max_altitude: Option<i32>,
    /// Cost for this tow tier
    pub cost: BigDecimal,
    /// User who last modified this fee tier
    pub modified_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel model for the club_tow_fees table - used for database operations
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::club_tow_fees)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ClubTowFeeModel {
    pub id: Uuid,
    pub club_id: Uuid,
    pub max_altitude: Option<i32>,
    pub cost: BigDecimal,
    pub modified_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert model for new club tow fees
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::club_tow_fees)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewClubTowFee {
    pub club_id: Uuid,
    pub max_altitude: Option<i32>,
    pub cost: BigDecimal,
    pub modified_by: Uuid,
}

/// Update model for existing club tow fees
#[derive(Debug, Clone, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::club_tow_fees)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateClubTowFee {
    pub max_altitude: Option<i32>,
    pub cost: BigDecimal,
    pub modified_by: Uuid,
}

/// Conversion from ClubTowFeeModel (database model) to ClubTowFee (API model)
impl From<ClubTowFeeModel> for ClubTowFee {
    fn from(model: ClubTowFeeModel) -> Self {
        Self {
            id: model.id,
            club_id: model.club_id,
            max_altitude: model.max_altitude,
            cost: model.cost,
            modified_by: model.modified_by,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Conversion from ClubTowFee (API model) to ClubTowFeeModel (database model)
impl From<ClubTowFee> for ClubTowFeeModel {
    fn from(fee: ClubTowFee) -> Self {
        Self {
            id: fee.id,
            club_id: fee.club_id,
            max_altitude: fee.max_altitude,
            cost: fee.cost,
            modified_by: fee.modified_by,
            created_at: fee.created_at,
            updated_at: fee.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_new_club_tow_fee_creation() {
        let club_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let cost = BigDecimal::from_str("45.00").unwrap();

        let new_fee = NewClubTowFee {
            club_id,
            max_altitude: Some(2000),
            cost: cost.clone(),
            modified_by: user_id,
        };

        assert_eq!(new_fee.club_id, club_id);
        assert_eq!(new_fee.max_altitude, Some(2000));
        assert_eq!(new_fee.cost, cost);
        assert_eq!(new_fee.modified_by, user_id);
    }

    #[test]
    fn test_fallback_tier_with_null_altitude() {
        let club_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let cost = BigDecimal::from_str("75.00").unwrap();

        let fallback_fee = NewClubTowFee {
            club_id,
            max_altitude: None, // Fallback tier for anything above highest altitude
            cost,
            modified_by: user_id,
        };

        assert_eq!(fallback_fee.max_altitude, None);
    }
}
