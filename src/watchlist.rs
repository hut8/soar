use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Watchlist entry response (returned to API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistEntry {
    pub user_id: Uuid,
    pub aircraft_id: Uuid,
    pub send_email: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to add aircraft to watchlist
#[derive(Debug, Deserialize)]
pub struct AddToWatchlistRequest {
    pub aircraft_id: Uuid,
    #[serde(default)]
    pub send_email: bool,
}

/// Request to update watchlist email preference
#[derive(Debug, Deserialize)]
pub struct UpdateWatchlistRequest {
    pub send_email: bool,
}
