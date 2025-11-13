use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use tracing::debug;
use uuid::Uuid;

use crate::server_messages::ServerMessage;
use crate::web::PgPool;

// Diesel model for inserting new server messages
#[derive(Insertable)]
#[diesel(table_name = crate::schema::server_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewServerMessage {
    id: Uuid,
    software: String,
    server_timestamp: DateTime<Utc>,
    received_at: DateTime<Utc>,
    server_name: String,
    server_endpoint: String,
    lag: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<&ServerMessage> for NewServerMessage {
    fn from(message: &ServerMessage) -> Self {
        Self {
            id: message.id,
            software: message.software.clone(),
            server_timestamp: message.server_timestamp,
            received_at: message.received_at,
            server_name: message.server_name.clone(),
            server_endpoint: message.server_endpoint.clone(),
            lag: message.lag,
            created_at: message.created_at,
            updated_at: message.updated_at,
        }
    }
}

// Diesel model for querying server messages
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::server_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ServerMessageRow {
    id: Uuid,
    software: String,
    server_timestamp: DateTime<Utc>,
    received_at: DateTime<Utc>,
    server_name: String,
    server_endpoint: String,
    lag: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ServerMessageRow> for ServerMessage {
    fn from(row: ServerMessageRow) -> Self {
        Self {
            id: row.id,
            software: row.software,
            server_timestamp: row.server_timestamp,
            received_at: row.received_at,
            server_name: row.server_name,
            server_endpoint: row.server_endpoint,
            lag: row.lag,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Clone)]
pub struct ServerMessagesRepository {
    pool: PgPool,
}

impl ServerMessagesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new server message into the database
    pub async fn insert(&self, message: &ServerMessage) -> Result<()> {
        use crate::schema::server_messages::dsl::*;

        let new_message = NewServerMessage::from(message);
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::insert_into(server_messages)
                .values(&new_message)
                .execute(&mut conn)?;
            Ok::<(), anyhow::Error>(())
        })
        .await??;

        debug!("Inserted server message: {:?}", message);
        Ok(())
    }
}
