use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
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
}
