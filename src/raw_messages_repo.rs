use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;
use uuid::Uuid;

use crate::web::PgPool;

/// Message source enum - distinguishes between protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[serde(rename_all = "lowercase")]
#[db_enum(existing_type_path = "crate::schema::sql_types::MessageSource")]
pub enum MessageSourceType {
    /// APRS/OGN message (text-based, UTF-8)
    Aprs,
    /// Beast protocol message (binary ADS-B frames)
    Beast,
    /// SBS-1 BaseStation message (CSV format)
    Sbs,
}

/// Raw message with source type - used for API responses
/// This struct properly handles both APRS (text) and ADS-B (binary) messages
#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::raw_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RawMessageWithSource {
    pub id: Uuid,
    #[diesel(deserialize_as = Vec<u8>)]
    pub raw_message: Vec<u8>,
    pub received_at: DateTime<Utc>,
    pub receiver_id: Option<Uuid>,
    pub unparsed: Option<String>,
    pub raw_message_hash: Vec<u8>,
    pub source: MessageSourceType,
}

/// API response for a raw message
/// For APRS: raw_message is UTF-8 text
/// For Beast: raw_message is hex-encoded binary
/// For SBS: raw_message is UTF-8 CSV text
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMessageResponse {
    pub id: Uuid,
    pub raw_message: String,
    pub source: MessageSourceType,
    pub received_at: DateTime<Utc>,
    pub receiver_id: Option<Uuid>,
    /// Pretty-printed Rust debug format of the decoded/parsed message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_format: Option<String>,
}

impl From<RawMessageWithSource> for RawMessageResponse {
    fn from(msg: RawMessageWithSource) -> Self {
        let (raw_message, debug_format) = match msg.source {
            MessageSourceType::Aprs => {
                // APRS is text - decode as UTF-8 (lossy for safety)
                let text = String::from_utf8_lossy(&msg.raw_message).to_string();

                // Try to parse and get debug format
                let debug_fmt = ogn_parser::parse(&text)
                    .map(|parsed| format!("{:#?}", parsed))
                    .ok();

                (text, debug_fmt)
            }
            MessageSourceType::Beast => {
                // Beast is binary - encode as hex
                let hex_encoded = hex::encode(&msg.raw_message);

                // Try to decode and get debug format
                let debug_fmt =
                    crate::beast::decoder::decode_beast_frame(&msg.raw_message, msg.received_at)
                        .map(|decoded| format!("{:#?}", decoded.message))
                        .ok();

                (hex_encoded, debug_fmt)
            }
            MessageSourceType::Sbs => {
                // SBS is text CSV - decode as UTF-8 (lossy for safety)
                let text = String::from_utf8_lossy(&msg.raw_message).to_string();

                // Try to parse and get debug format
                let debug_fmt = crate::sbs::parser::parse_sbs_message(&text)
                    .map(|parsed| format!("{:#?}", parsed))
                    .ok();

                (text, debug_fmt)
            }
        };

        RawMessageResponse {
            id: msg.id,
            raw_message,
            source: msg.source,
            received_at: msg.received_at,
            receiver_id: msg.receiver_id,
            debug_format,
        }
    }
}

// Diesel model for inserting new Beast messages (using raw SQL for enum)
#[derive(Clone)]
pub struct NewBeastMessage {
    pub id: Uuid,
    pub raw_message: Vec<u8>, // Binary Beast frame
    pub received_at: DateTime<Utc>,
    pub receiver_id: Option<Uuid>, // NULL for ADS-B/Beast - no receiver concept
    pub unparsed: Option<String>,
    pub raw_message_hash: Vec<u8>,
}

impl NewBeastMessage {
    /// Create a new Beast message with computed hash
    /// receiver_id should be None for ADS-B messages (no receiver concept)
    pub fn new(
        raw_message: Vec<u8>, // Binary Beast frame
        received_at: DateTime<Utc>,
        receiver_id: Option<Uuid>,
        unparsed: Option<String>,
    ) -> Self {
        // Compute SHA-256 hash of raw message for deduplication
        let mut hasher = Sha256::new();
        hasher.update(&raw_message);
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

// Diesel model for inserting new SBS messages (using raw SQL for enum)
#[derive(Clone)]
pub struct NewSbsMessage {
    pub id: Uuid,
    pub raw_message: Vec<u8>, // UTF-8 encoded SBS CSV line
    pub received_at: DateTime<Utc>,
    pub receiver_id: Option<Uuid>, // NULL for SBS - no receiver concept
    pub unparsed: Option<String>,
    pub raw_message_hash: Vec<u8>,
}

impl NewSbsMessage {
    /// Create a new SBS message with computed hash
    /// receiver_id should be None for SBS messages (no receiver concept)
    pub fn new(
        raw_message: Vec<u8>, // UTF-8 encoded SBS CSV line
        received_at: DateTime<Utc>,
        receiver_id: Option<Uuid>,
        unparsed: Option<String>,
    ) -> Self {
        // Compute SHA-256 hash of raw message for deduplication
        let mut hasher = Sha256::new();
        hasher.update(&raw_message);
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

// Legacy APRS-specific struct for backward compatibility
#[derive(Insertable)]
#[diesel(table_name = crate::schema::raw_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAprsMessage {
    pub id: Uuid,
    pub raw_message: Vec<u8>, // UTF-8 encoded APRS text
    pub received_at: DateTime<Utc>,
    pub receiver_id: Uuid,
    pub unparsed: Option<String>,
    pub raw_message_hash: Vec<u8>,
}

impl NewAprsMessage {
    /// Create a new APRS message with computed hash
    /// Accepts ASCII/UTF-8 text and stores as bytes
    pub fn new(
        raw_message: String, // APRS text message
        received_at: DateTime<Utc>,
        receiver_id: Uuid,
        unparsed: Option<String>,
    ) -> Self {
        // Convert text to UTF-8 bytes for storage
        let message_bytes = raw_message.into_bytes();

        // Compute SHA-256 hash of raw message for deduplication
        let mut hasher = Sha256::new();
        hasher.update(&message_bytes);
        let hash = hasher.finalize().to_vec();

        Self {
            id: Uuid::now_v7(),
            raw_message: message_bytes,
            received_at,
            receiver_id,
            unparsed,
            raw_message_hash: hash,
        }
    }
}

// Diesel model for querying APRS messages
// Note: raw_message is stored as BYTEA (UTF-8 encoded text for APRS)
// For JSON serialization, raw_message is automatically decoded to UTF-8 string
#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::raw_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AprsMessage {
    pub id: Uuid,
    #[diesel(deserialize_as = Vec<u8>)]
    pub raw_message: Vec<u8>, // UTF-8 encoded APRS text (stored as BYTEA)
    pub received_at: DateTime<Utc>,
    pub receiver_id: Option<Uuid>, // NULL for ADS-B/Beast messages
    pub unparsed: Option<String>,
    pub raw_message_hash: Vec<u8>,
}

impl AprsMessage {
    /// Decode raw_message bytes to UTF-8 string
    /// Returns lossy conversion if invalid UTF-8 is encountered
    pub fn raw_message_text(&self) -> String {
        String::from_utf8_lossy(&self.raw_message).to_string()
    }
}

// Custom serialization for JSON APIs - decode raw_message to string
impl Serialize for AprsMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AprsMessage", 6)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("raw_message", &self.raw_message_text())?; // Decode to string
        state.serialize_field("received_at", &self.received_at)?;
        state.serialize_field("receiver_id", &self.receiver_id)?;
        state.serialize_field("unparsed", &self.unparsed)?;
        state.serialize_field("raw_message_hash", &hex::encode(&self.raw_message_hash))?; // Hex encode hash
        state.end()
    }
}

// Custom deserialization for JSON APIs - encode string to bytes
impl<'de> Deserialize<'de> for AprsMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AprsMessageHelper {
            id: Uuid,
            raw_message: String, // Expect string in JSON
            received_at: DateTime<Utc>,
            receiver_id: Option<Uuid>, // Nullable for ADS-B messages
            unparsed: Option<String>,
            raw_message_hash: String, // Hex-encoded in JSON
        }

        let helper = AprsMessageHelper::deserialize(deserializer)?;
        Ok(AprsMessage {
            id: helper.id,
            raw_message: helper.raw_message.into_bytes(), // Convert to bytes
            received_at: helper.received_at,
            receiver_id: helper.receiver_id,
            unparsed: helper.unparsed,
            raw_message_hash: hex::decode(&helper.raw_message_hash)
                .map_err(serde::de::Error::custom)?,
        })
    }
}

/// Unified repository for raw messages (both APRS and Beast/ADS-B)
/// Provides type-safe methods to insert and query messages from the raw_messages table
#[derive(Clone)]
pub struct RawMessagesRepository {
    pool: PgPool,
}

impl RawMessagesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new APRS message into the database
    /// Returns the ID of the inserted message
    /// On duplicate (redelivery after crash), returns the existing message ID
    pub async fn insert_aprs(&self, new_message: NewAprsMessage) -> Result<Uuid> {
        use crate::schema::raw_messages::dsl::*;

        let message_id = new_message.id;
        let pool = self.pool.clone();
        let receiver = new_message.receiver_id;
        let timestamp = new_message.received_at;
        let hash = new_message.raw_message_hash.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            match diesel::insert_into(raw_messages)
                .values(&new_message)
                .execute(&mut conn)
            {
                Ok(_) => {
                    metrics::counter!("aprs.messages.inserted_total").increment(1);
                    Ok::<Uuid, anyhow::Error>(message_id)
                }
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    // Duplicate message on redelivery - this is expected after crashes
                    debug!("Duplicate aprs_message detected on redelivery");
                    metrics::counter!("aprs.messages.duplicate_on_redelivery_total").increment(1);

                    // Find existing message ID by natural key
                    let existing = raw_messages
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

    /// Insert a new Beast (ADS-B) message into the database
    /// Returns the ID of the inserted message
    /// On duplicate (redelivery after crash), returns the existing message ID
    pub async fn insert_beast(&self, new_message: NewBeastMessage) -> Result<Uuid> {
        let message_id = new_message.id;
        let pool = self.pool.clone();
        let receiver = new_message.receiver_id;
        let timestamp = new_message.received_at;
        let hash = new_message.raw_message_hash.clone();
        let raw_msg = new_message.raw_message;
        let unparsed_val = new_message.unparsed;

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL to insert with enum value 'beast'
            let insert_result = diesel::sql_query(
                "INSERT INTO raw_messages (id, raw_message, received_at, receiver_id, unparsed, raw_message_hash, source)
                 VALUES ($1, $2, $3, $4, $5, $6, 'beast'::message_source)"
            )
            .bind::<diesel::sql_types::Uuid, _>(message_id)
            .bind::<diesel::sql_types::Bytea, _>(&raw_msg)  // Binary Beast frame
            .bind::<diesel::sql_types::Timestamptz, _>(timestamp)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(receiver)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&unparsed_val)
            .bind::<diesel::sql_types::Bytea, _>(&hash)
            .execute(&mut conn);

            match insert_result {
                Ok(_) => {
                    metrics::counter!("beast.messages.inserted_total").increment(1);
                    Ok::<Uuid, anyhow::Error>(message_id)
                }
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    // Duplicate message on redelivery - this is expected after crashes
                    debug!("Duplicate beast message detected on redelivery");
                    metrics::counter!("beast.messages.duplicate_on_redelivery_total").increment(1);

                    // Find existing message ID by natural key
                    use crate::schema::raw_messages::dsl::*;
                    let query = if let Some(recv_id) = receiver {
                        raw_messages
                            .filter(receiver_id.eq(recv_id))
                            .filter(received_at.eq(timestamp))
                            .filter(raw_message_hash.eq(&hash))
                            .select(id)
                            .into_boxed()
                    } else {
                        raw_messages
                            .filter(receiver_id.is_null())
                            .filter(received_at.eq(timestamp))
                            .filter(raw_message_hash.eq(&hash))
                            .select(id)
                            .into_boxed()
                    };

                    let existing = query.first::<Uuid>(&mut conn)?;
                    Ok(existing)
                }
                Err(e) => Err(e.into()),
            }
        })
        .await??;

        Ok(result)
    }

    /// Insert a batch of Beast messages into the database
    /// Uses a transaction for atomicity
    /// Note: Duplicates are not handled in batch mode - each insert will fail on primary key violation
    pub async fn insert_beast_batch(&self, messages: &[NewBeastMessage]) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let pool = self.pool.clone();
        let messages_vec = messages.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            conn.transaction::<_, anyhow::Error, _>(|conn| {
                for message in &messages_vec {
                    // Use raw SQL to insert with enum value 'beast'
                    // No ON CONFLICT clause - let duplicates fail naturally on primary key
                    let insert_result = diesel::sql_query(
                        "INSERT INTO raw_messages (id, raw_message, received_at, receiver_id, unparsed, raw_message_hash, source)
                         VALUES ($1, $2, $3, $4, $5, $6, 'beast'::message_source)"
                    )
                    .bind::<diesel::sql_types::Uuid, _>(message.id)
                    .bind::<diesel::sql_types::Bytea, _>(&message.raw_message)
                    .bind::<diesel::sql_types::Timestamptz, _>(message.received_at)
                    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(message.receiver_id)
                    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&message.unparsed)
                    .bind::<diesel::sql_types::Bytea, _>(&message.raw_message_hash)
                    .execute(conn)?;

                    if insert_result > 0 {
                        metrics::counter!("beast.messages.inserted_total").increment(1);
                    }
                }
                Ok(())
            })?;

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    /// Insert a new SBS message into the database
    /// Returns the ID of the inserted message
    /// On duplicate (redelivery after crash), returns the existing message ID
    pub async fn insert_sbs(&self, new_message: NewSbsMessage) -> Result<Uuid> {
        let message_id = new_message.id;
        let pool = self.pool.clone();
        let receiver = new_message.receiver_id;
        let timestamp = new_message.received_at;
        let hash = new_message.raw_message_hash.clone();
        let raw_msg = new_message.raw_message;
        let unparsed_val = new_message.unparsed;

        let result = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Use raw SQL to insert with enum value 'sbs'
            let insert_result = diesel::sql_query(
                "INSERT INTO raw_messages (id, raw_message, received_at, receiver_id, unparsed, raw_message_hash, source)
                 VALUES ($1, $2, $3, $4, $5, $6, 'sbs'::message_source)"
            )
            .bind::<diesel::sql_types::Uuid, _>(message_id)
            .bind::<diesel::sql_types::Bytea, _>(&raw_msg)  // UTF-8 encoded SBS CSV
            .bind::<diesel::sql_types::Timestamptz, _>(timestamp)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(receiver)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&unparsed_val)
            .bind::<diesel::sql_types::Bytea, _>(&hash)
            .execute(&mut conn);

            match insert_result {
                Ok(_) => {
                    metrics::counter!("sbs.messages.inserted_total").increment(1);
                    Ok::<Uuid, anyhow::Error>(message_id)
                }
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    // Duplicate message on redelivery - this is expected after crashes
                    debug!("Duplicate SBS message detected on redelivery");
                    metrics::counter!("sbs.messages.duplicate_on_redelivery_total").increment(1);

                    // Find existing message ID by natural key
                    use crate::schema::raw_messages::dsl::*;
                    let query = if let Some(recv_id) = receiver {
                        raw_messages
                            .filter(receiver_id.eq(recv_id))
                            .filter(received_at.eq(timestamp))
                            .filter(raw_message_hash.eq(&hash))
                            .select(id)
                            .into_boxed()
                    } else {
                        raw_messages
                            .filter(receiver_id.is_null())
                            .filter(received_at.eq(timestamp))
                            .filter(raw_message_hash.eq(&hash))
                            .select(id)
                            .into_boxed()
                    };

                    let existing = query.first::<Uuid>(&mut conn)?;
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
        use crate::schema::raw_messages::dsl::*;

        let pool = self.pool.clone();
        let offset = (page - 1) * per_page;

        // Calculate 24 hours ago
        let twenty_four_hours_ago = Utc::now() - Duration::hours(24);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            // Get total count
            let total_count: i64 = raw_messages
                .filter(receiver_id.eq(receiver_uuid))
                .filter(received_at.ge(twenty_four_hours_ago))
                .count()
                .get_result(&mut conn)?;

            // Get paginated messages
            let messages: Vec<AprsMessage> = raw_messages
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

    /// Get a single raw message by ID
    pub async fn get_by_id(&self, message_id: Uuid) -> Result<Option<AprsMessage>> {
        use crate::schema::raw_messages::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let message = raw_messages
                .filter(id.eq(message_id))
                .select(AprsMessage::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<AprsMessage>, anyhow::Error>(message)
        })
        .await?
    }

    /// Get multiple raw messages by their IDs
    pub async fn get_by_ids(&self, message_ids: Vec<Uuid>) -> Result<Vec<AprsMessage>> {
        use crate::schema::raw_messages::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let messages = raw_messages
                .filter(id.eq_any(message_ids))
                .select(AprsMessage::as_select())
                .load(&mut conn)?;

            Ok::<Vec<AprsMessage>, anyhow::Error>(messages)
        })
        .await?
    }

    /// Get a raw message by ID with proper encoding based on source type
    /// Returns APRS messages as UTF-8 text, ADS-B messages as hex-encoded binary
    pub async fn get_message_response_by_id(
        &self,
        message_id: Uuid,
    ) -> Result<Option<RawMessageResponse>> {
        use crate::schema::raw_messages::dsl::*;

        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let message = raw_messages
                .filter(id.eq(message_id))
                .select(RawMessageWithSource::as_select())
                .first(&mut conn)
                .optional()?;

            Ok::<Option<RawMessageResponse>, anyhow::Error>(message.map(RawMessageResponse::from))
        })
        .await?
    }
}

// Type aliases for backward compatibility during migration
pub type AprsMessagesRepository = RawMessagesRepository;
pub type BeastMessagesRepository = RawMessagesRepository;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use diesel::connection::SimpleConnection;
    use diesel::r2d2::{ConnectionManager, Pool};
    use serial_test::serial;

    /// Helper to create a test database pool
    /// Uses TEST_DATABASE_URL environment variable or defaults to local test database
    fn create_test_pool() -> PgPool {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/soar_test".to_string());

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        Pool::builder()
            .max_size(5)
            .build(manager)
            .expect("Failed to create test pool")
    }

    /// Helper to clean up test data between tests
    ///
    /// NOTE: Assumes migrations have already been run (as in CI).
    /// To run tests locally, first run migrations on your test database:
    ///   diesel migration run --database-url postgresql://localhost/soar_test
    fn cleanup_test_data(pool: &PgPool) {
        let mut conn = pool.get().expect("Failed to get connection");

        // CRITICAL: TRUNCATE in dependency order (children first) to avoid deadlocks
        // NEVER use CASCADE - it creates complex lock hierarchies on TimescaleDB hypertables
        // that deadlock with concurrent INSERT operations
        //
        // Dependency order:
        // 1. fixes (references raw_messages)
        // 2. receiver_statuses (references raw_messages)
        // 3. raw_messages (hypertable, references receivers)
        // 4. receivers (parent table)
        let _ = conn.batch_execute("TRUNCATE TABLE fixes");
        let _ = conn.batch_execute("TRUNCATE TABLE receiver_statuses");
        let _ = conn.batch_execute("TRUNCATE TABLE raw_messages");
        let _ = conn.batch_execute("TRUNCATE TABLE receivers");
    }

    #[tokio::test]
    #[serial]
    async fn test_insert_and_get_by_id() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let receiver_id = Uuid::new_v4();
        let callsign = format!("TEST{}", &receiver_id.to_string()[..8]);
        let mut message_id = Uuid::nil();

        // Insert receiver and message in a single transaction
        {
            use diesel::Connection;
            let mut conn = pool.get().expect("Failed to get connection");
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                // Insert receiver using direct SQL
                diesel::sql_query("INSERT INTO receivers (id, callsign) VALUES ($1, $2)")
                    .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                    .bind::<diesel::sql_types::Text, _>(&callsign)
                    .execute(conn)?;

                // Insert message using Diesel
                let new_message = NewAprsMessage::new(
                    "TEST>APRS:>Test message".to_string(),
                    Utc::now(),
                    receiver_id,
                    None,
                );
                message_id = new_message.id;
                diesel::insert_into(crate::schema::raw_messages::table)
                    .values(&new_message)
                    .execute(conn)?;

                Ok(())
            })
            .expect("Failed to insert test data");
            // Explicitly drop connection before using repo
            drop(conn);
        }

        // Now use repo for querying
        let repo = AprsMessagesRepository::new(pool.clone());
        let retrieved = repo
            .get_by_id(message_id)
            .await
            .expect("Failed to get by ID");

        assert!(retrieved.is_some());
        let message = retrieved.unwrap();
        assert_eq!(message.id, message_id);
        assert_eq!(message.raw_message_text(), "TEST>APRS:>Test message");
        assert_eq!(message.receiver_id, Some(receiver_id));
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
    async fn test_get_by_ids_multiple() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let receiver_id = Uuid::new_v4();
        let callsign = format!("TEST{}", &receiver_id.to_string()[..8]);
        let mut message_ids: Vec<Uuid> = Vec::new();

        // Insert receiver and messages in a single transaction
        {
            use diesel::Connection;
            let mut conn = pool.get().expect("Failed to get connection");
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                // Insert receiver using direct SQL
                diesel::sql_query("INSERT INTO receivers (id, callsign) VALUES ($1, $2)")
                    .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                    .bind::<diesel::sql_types::Text, _>(&callsign)
                    .execute(conn)?;

                // Insert multiple messages using batch insert to avoid deadlocks
                // Use well-spaced timestamps (seconds apart) to avoid TimescaleDB chunk conflicts
                // Use a fixed base time to avoid issues with chunk boundaries
                let base_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
                let new_messages: Vec<NewAprsMessage> = (0..3)
                    .map(|i| {
                        NewAprsMessage::new(
                            format!("TEST{}>APRS:>Test message {}", i, i),
                            base_time + chrono::Duration::seconds(i as i64 * 10),
                            receiver_id,
                            None,
                        )
                    })
                    .collect();

                // Store message IDs for later verification
                message_ids.extend(new_messages.iter().map(|m| m.id));

                // Batch insert all messages at once to avoid lock contention
                diesel::insert_into(crate::schema::raw_messages::table)
                    .values(&new_messages)
                    .execute(conn)?;

                Ok(())
            })
            .expect("Failed to insert test data");
            // Explicitly drop connection before using repo
            drop(conn);
        }

        // Now use repo for querying
        let repo = AprsMessagesRepository::new(pool.clone());
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
    #[serial]
    async fn test_get_by_ids_partial_match() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let receiver_id = Uuid::new_v4();
        let callsign = format!("TEST{}", &receiver_id.to_string()[..8]);
        let mut existing_id = Uuid::nil();

        // Insert receiver and message in a single transaction
        {
            use diesel::Connection;
            let mut conn = pool.get().expect("Failed to get connection");
            conn.transaction::<_, anyhow::Error, _>(|conn| {
                // Insert receiver using direct SQL
                diesel::sql_query("INSERT INTO receivers (id, callsign) VALUES ($1, $2)")
                    .bind::<diesel::sql_types::Uuid, _>(receiver_id)
                    .bind::<diesel::sql_types::Text, _>(&callsign)
                    .execute(conn)?;

                // Insert message using Diesel
                let new_message = NewAprsMessage::new(
                    "TEST>APRS:>Existing message".to_string(),
                    Utc::now(),
                    receiver_id,
                    None,
                );
                existing_id = new_message.id;
                diesel::insert_into(crate::schema::raw_messages::table)
                    .values(&new_message)
                    .execute(conn)?;

                Ok(())
            })
            .expect("Failed to insert test data");
            // Explicitly drop connection before using repo
            drop(conn);
        }

        // Now use repo for querying
        let repo = AprsMessagesRepository::new(pool.clone());

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
    #[serial]
    async fn test_get_by_ids_empty_list() {
        let pool = create_test_pool();
        cleanup_test_data(&pool);

        let repo = AprsMessagesRepository::new(pool);

        let messages = repo.get_by_ids(vec![]).await.expect("Failed to get by IDs");

        assert_eq!(messages.len(), 0);
    }
}
