use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::web::PgPool;

// Diesel model for inserting new APRS messages
#[derive(Insertable)]
#[diesel(table_name = crate::schema::aprs_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAprsMessage {
    pub id: Uuid,
    pub raw_message: String,
    pub received_at: DateTime<Utc>,
    pub receiver_id: Uuid,
    pub unparsed: Option<String>,
}

// Diesel model for querying APRS messages
#[derive(Queryable, Selectable, Serialize, Debug)]
#[diesel(table_name = crate::schema::aprs_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AprsMessage {
    pub id: Uuid,
    pub raw_message: String,
    pub received_at: DateTime<Utc>,
    pub receiver_id: Uuid,
    pub unparsed: Option<String>,
}

#[derive(Clone)]
pub struct AprsMessagesRepository {
    pool: PgPool,
}

impl AprsMessagesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new APRS message into the database
    /// Returns the ID of the inserted message
    pub async fn insert(&self, new_message: NewAprsMessage) -> Result<Uuid> {
        use crate::schema::aprs_messages::dsl::*;

        let message_id = new_message.id;
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(aprs_messages)
                .values(&new_message)
                .execute(&mut conn)?;
            Ok::<Uuid, anyhow::Error>(message_id)
        })
        .await??;

        Ok(message_id)
    }

    /// Get paginated raw messages for a receiver from the last 24 hours
    /// Returns (messages, total_count)
    pub async fn get_messages_by_receiver_paginated(
        &self,
        receiver_uuid: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<AprsMessage>, i64)> {
        use crate::schema::aprs_messages::dsl::*;

        let pool = self.pool.clone();
        let offset = (page - 1) * per_page;

        // Calculate 24 hours ago
        let twenty_four_hours_ago = Utc::now() - Duration::hours(24);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Get total count
            let total_count: i64 = aprs_messages
                .filter(receiver_id.eq(receiver_uuid))
                .filter(received_at.ge(twenty_four_hours_ago))
                .count()
                .get_result(&mut conn)?;

            // Get paginated messages
            let messages: Vec<AprsMessage> = aprs_messages
                .filter(receiver_id.eq(receiver_uuid))
                .filter(received_at.ge(twenty_four_hours_ago))
                .order(received_at.desc())
                .limit(per_page)
                .offset(offset)
                .select(AprsMessage::as_select())
                .load(&mut conn)?;

            Ok::<(Vec<AprsMessage>, i64), anyhow::Error>((messages, total_count))
        })
        .await?
    }
}
