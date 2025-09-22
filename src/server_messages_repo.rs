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

    /// Get recent server messages for a specific server
    pub async fn get_recent_by_server(
        &self,
        server_name_filter: &str,
        limit: i64,
    ) -> Result<Vec<ServerMessage>> {
        use crate::schema::server_messages::dsl::*;

        let pool = self.pool.clone();
        let server_name_filter = server_name_filter.to_string();

        let rows = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let rows: Vec<ServerMessageRow> = server_messages
                .filter(server_name.eq(server_name_filter))
                .order(server_timestamp.desc())
                .limit(limit)
                .select(ServerMessageRow::as_select())
                .load(&mut conn)?;
            Ok::<Vec<ServerMessageRow>, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows.into_iter().map(ServerMessage::from).collect())
    }

    /// Get all recent server messages across all servers
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<ServerMessage>> {
        use crate::schema::server_messages::dsl::*;

        let pool = self.pool.clone();

        let rows = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let rows: Vec<ServerMessageRow> = server_messages
                .order(server_timestamp.desc())
                .limit(limit)
                .select(ServerMessageRow::as_select())
                .load(&mut conn)?;
            Ok::<Vec<ServerMessageRow>, anyhow::Error>(rows)
        })
        .await??;

        Ok(rows.into_iter().map(ServerMessage::from).collect())
    }

    /// Get server message count for a specific server
    pub async fn count_by_server(&self, server_name_filter: &str) -> Result<i64> {
        use crate::schema::server_messages::dsl::*;

        let pool = self.pool.clone();
        let server_name_filter = server_name_filter.to_string();

        let count = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count = server_messages
                .filter(server_name.eq(server_name_filter))
                .count()
                .get_result(&mut conn)?;
            Ok::<i64, anyhow::Error>(count)
        })
        .await??;

        Ok(count)
    }
}
