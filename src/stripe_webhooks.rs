use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Diesel model for the stripe_webhook_events table
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::stripe_webhook_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct StripeWebhookEventModel {
    pub id: Uuid,
    pub stripe_event_id: String,
    pub event_type: String,
    pub processed: bool,
    pub processing_error: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Insert model for new webhook events
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::stripe_webhook_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewStripeWebhookEvent {
    pub stripe_event_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}
