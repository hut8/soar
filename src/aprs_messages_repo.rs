use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;
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
    pub raw_message_hash: Vec<u8>,
}

impl NewAprsMessage {
    /// Create a new APRS message with computed hash
    pub fn new(
        raw_message: String,
        received_at: DateTime<Utc>,
        receiver_id: Uuid,
        unparsed: Option<String>,
    ) -> Self {
        // Compute SHA-256 hash of raw message for deduplication
        let mut hasher = Sha256::new();
        hasher.update(raw_message.as_bytes());
        let hash = hasher.finalize().to_vec();

        Self {
            id: Uuid::now_v7(),
            raw_message,
            received_at,
            receiver_id,
            unparsed,
            raw_message_hash: hash,
        }
    }
}

// Diesel model for querying APRS messages
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = crate::schema::aprs_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AprsMessage {
    pub id: Uuid,
    pub raw_message: String,
    pub received_at: DateTime<Utc>,
    pub receiver_id: Uuid,
    pub unparsed: Option<String>,
    pub raw_message_hash: Vec<u8>,
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
    /// On duplicate (redelivery after crash), returns the existing message ID
    pub async fn insert(&self, new_message: NewAprsMessage) -> Result<Uuid> {
        use crate::schema::aprs_messages::dsl::*;

        let message_id = new_message.id;
        let pool = self.pool.clone();
        let receiver = new_message.receiver_id;
        let timestamp = new_message.received_at;
        let hash = new_message.raw_message_hash.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            match diesel::insert_into(aprs_messages)
                .values(&new_message)
                .execute(&mut conn)
            {
                Ok(_) => {
                    metrics::counter!("aprs.messages.inserted").increment(1);
                    Ok::<Uuid, anyhow::Error>(message_id)
                }
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    // Duplicate message on redelivery - this is expected after crashes
                    debug!("Duplicate aprs_message detected on redelivery");
                    metrics::counter!("aprs.messages.duplicate_on_redelivery").increment(1);

                    // Find existing message ID by natural key
                    let existing = aprs_messages
                        .filter(receiver_id.eq(receiver))
                        .filter(received_at.eq(timestamp))
                        .filter(raw_message_hash.eq(&hash))
                        .select(id)
                        .first::<Uuid>(&mut conn)?;

                    Ok(existing)
                }
                Err(e) => Err(e.into()),
            }
        })
        .await??;

        Ok(result)
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

    /// Get a single APRS message by ID
    pub async fn get_by_id(&self, message_id: Uuid) -> Result<Option<AprsMessage>> {
        use crate::schema::aprs_messages::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let message = aprs_messages
                .filter(id.eq(message_id))
                .select(AprsMessage::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<AprsMessage>, anyhow::Error>(message)
        })
        .await?
    }

    /// Get multiple APRS messages by their IDs
    pub async fn get_by_ids(&self, message_ids: Vec<Uuid>) -> Result<Vec<AprsMessage>> {
        use crate::schema::aprs_messages::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let messages = aprs_messages
                .filter(id.eq_any(message_ids))
                .select(AprsMessage::as_select())
                .load(&mut conn)?;

            Ok::<Vec<AprsMessage>, anyhow::Error>(messages)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use diesel::connection::SimpleConnection;
    use diesel::r2d2::{ConnectionManager, Pool};

    /// Helper to create a test database pool
    /// Uses DATABASE_URL environment variable (set in CI) or defaults to local test database
    fn create_test_pool() -> PgPool {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for tests (e.g., postgres://postgres:postgres@localhost:5432/soar_test)");

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("Failed to create test pool")
    }

    /// Helper to clean up test data between tests
    /// Assumes migrations have already been run (as in CI)
    fn cleanup_test_data(pool: &PgPool) {
        let mut conn = pool.get().expect("Failed to get connection");

        // Clean up test data (migrations have already created the schema)
        conn.batch_execute(
            r#"
            DELETE FROM aprs_messages;
            DELETE FROM receivers;
            "#,
        )
        .expect("Failed to clean up test data");
    }

    #[tokio::test]
    async fn test_insert_and_get_by_id() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let repo = AprsMessagesRepository::new(pool.clone());

        // Create a test receiver with unique callsign
        let receiver_id = Uuid::new_v4();
        let callsign = format!("TEST{}", &receiver_id.to_string()[..8]);
        {
            let mut conn = pool.get().expect("Failed to get connection");
            diesel::sql_query("INSERT INTO receivers (id, callsign) VALUES ($1, $2)")
                .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                .bind::<diesel::sql_types::Text, _>(&callsign)
                .execute(&mut conn)
                .expect("Failed to insert test receiver");
        }

        // Insert a test message
        let new_message = NewAprsMessage::new(
            "TEST>APRS:>Test message".to_string(),
            Utc::now(),
            receiver_id,
            None,
        );
        let message_id = new_message.id;

        let inserted_id = repo.insert(new_message).await.expect("Failed to insert");
        assert_eq!(inserted_id, message_id);

        // Retrieve the message by ID
        let retrieved = repo
            .get_by_id(message_id)
            .await
            .expect("Failed to get by ID");

        assert!(retrieved.is_some());
        let message = retrieved.unwrap();
        assert_eq!(message.id, message_id);
        assert_eq!(message.raw_message, "TEST>APRS:>Test message");
        assert_eq!(message.receiver_id, receiver_id);
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let repo = AprsMessagesRepository::new(pool);

        let non_existent_id = Uuid::new_v4();
        let result = repo
            .get_by_id(non_existent_id)
            .await
            .expect("Query should succeed");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_ids_multiple() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let repo = AprsMessagesRepository::new(pool.clone());

        // Create a test receiver with unique callsign
        let receiver_id = Uuid::new_v4();
        let callsign = format!("TEST{}", &receiver_id.to_string()[..8]);
        {
            let mut conn = pool.get().expect("Failed to get connection");
            diesel::sql_query("INSERT INTO receivers (id, callsign) VALUES ($1, $2)")
                .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                .bind::<diesel::sql_types::Text, _>(&callsign)
                .execute(&mut conn)
                .expect("Failed to insert test receiver");
        }

        // Insert multiple test messages
        let mut message_ids: Vec<Uuid> = Vec::new();

        for i in 0..3 {
            let new_message = NewAprsMessage::new(
                format!("TEST{}>APRS:>Test message {}", i, i),
                Utc::now(),
                receiver_id,
                None,
            );
            message_ids.push(new_message.id);
            repo.insert(new_message).await.expect("Failed to insert");
        }

        // Retrieve all messages by their IDs
        let messages = repo
            .get_by_ids(message_ids.clone())
            .await
            .expect("Failed to get by IDs");

        assert_eq!(messages.len(), 3);

        // Verify all messages were retrieved
        for &id in &message_ids {
            assert!(messages.iter().any(|m| m.id == id));
        }
    }

    #[tokio::test]
    async fn test_get_by_ids_partial_match() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let repo = AprsMessagesRepository::new(pool.clone());

        // Create a test receiver with unique callsign
        let receiver_id = Uuid::new_v4();
        let callsign = format!("TEST{}", &receiver_id.to_string()[..8]);
        {
            let mut conn = pool.get().expect("Failed to get connection");
            diesel::sql_query("INSERT INTO receivers (id, callsign) VALUES ($1, $2)")
                .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                .bind::<diesel::sql_types::Text, _>(&callsign)
                .execute(&mut conn)
                .expect("Failed to insert test receiver");
        }

        // Insert one message
        let new_message = NewAprsMessage::new(
            "TEST>APRS:>Existing message".to_string(),
            Utc::now(),
            receiver_id,
            None,
        );
        let existing_id = new_message.id;
        repo.insert(new_message).await.expect("Failed to insert");

        // Request both existing and non-existing IDs
        let non_existing_id = Uuid::new_v4();
        let requested_ids = vec![existing_id, non_existing_id];

        let messages = repo
            .get_by_ids(requested_ids)
            .await
            .expect("Failed to get by IDs");

        // Should only return the existing message
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, existing_id);
    }

    #[tokio::test]
    async fn test_get_by_ids_empty_list() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let repo = AprsMessagesRepository::new(pool);

        let messages = repo.get_by_ids(vec![]).await.expect("Failed to get by IDs");

        assert_eq!(messages.len(), 0);
    }
}
