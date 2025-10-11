use crate::server_messages::ServerMessage;
use crate::server_messages_repo::ServerMessagesRepository;
use chrono::{DateTime, NaiveDateTime, Utc};
use tracing::{debug, trace};

/// Processor for handling APRS server comment/status messages
pub struct ServerStatusProcessor {
    /// Repository for storing server messages
    server_messages_repo: ServerMessagesRepository,
}

impl ServerStatusProcessor {
    /// Create a new ServerStatusProcessor
    pub fn new(server_messages_repo: ServerMessagesRepository) -> Self {
        Self {
            server_messages_repo,
        }
    }

    /// Process a server status message (line starting with #)
    pub async fn process_server_message(&self, raw_message: &str) {
        let received_at = Utc::now();

        // Parse server message format: # aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152
        if let Some(parsed) = self.parse_server_message(raw_message, received_at) {
            if let Err(e) = self.server_messages_repo.insert(&parsed).await {
                debug!("Failed to insert server message: {}", e);
            } else {
                trace!(
                    "Successfully stored server message from {}",
                    parsed.server_name
                );
            }
        } else {
            debug!("Failed to parse server message: {}", raw_message);
        }
    }

    /// Parse server message string into ServerMessage
    /// Expected format: # aprsc 2.1.15-gc67551b 22 Sep 2025 21:51:55 GMT GLIDERN1 51.178.19.212:10152
    fn parse_server_message(
        &self,
        raw_message: &str,
        received_at: DateTime<Utc>,
    ) -> Option<ServerMessage> {
        let trimmed = raw_message.trim_start_matches('#').trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        trace!("Parsing server message parts: {:?}", parts);

        // Need at least: software version, date (3 parts), time, GMT, server_name, endpoint
        if parts.len() < 8 {
            debug!(
                "Server message has {} parts, need at least 8: {}",
                parts.len(),
                raw_message
            );
            return None;
        }

        // Extract software (first two parts: "aprsc 2.1.15-gc67551b")
        let software = format!("{} {}", parts[0], parts[1]);

        // Extract timestamp parts: "22 Sep 2025 21:51:55 GMT"
        let date_time_str = format!(
            "{} {} {} {} {}",
            parts[2], parts[3], parts[4], parts[5], parts[6]
        );

        // Parse timestamp
        let server_timestamp =
            match NaiveDateTime::parse_from_str(&date_time_str, "%d %b %Y %H:%M:%S GMT") {
                Ok(naive_dt) => naive_dt.and_utc(),
                Err(_) => {
                    debug!("Failed to parse timestamp: {}", date_time_str);
                    return None;
                }
            };

        // Extract server name and endpoint
        let server_name = parts[7].to_string();
        let server_endpoint = parts[8].to_string();

        Some(ServerMessage::new(
            software,
            server_timestamp,
            received_at,
            server_name,
            server_endpoint,
        ))
    }
}
