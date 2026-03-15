use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use tracing::{error, info, warn};
use uuid::Uuid;
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushMessageBuilder,
};

use crate::push_subscriptions::FlightNotificationPayload;
use crate::push_subscriptions_repo::PushSubscriptionsRepository;

type PgPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct PushNotificationSender {
    pool: PgPool,
    vapid_private_key: Vec<u8>,
    vapid_subject: String,
}

impl PushNotificationSender {
    /// Try to create a new sender from environment variables.
    /// Returns None if VAPID keys are not configured (push notifications disabled).
    ///
    /// Required env vars:
    /// - `VAPID_PRIVATE_KEY_FILE`: Path to PEM-formatted ECDSA P-256 private key
    ///
    /// Optional:
    /// - `VAPID_SUBJECT`: Contact URI (default: mailto:admin@glider.flights)
    ///
    /// Note: `VAPID_PUBLIC_KEY` is also needed in the env for the web API endpoint
    /// (`GET /push/vapid-key`) but is not read here.
    pub fn from_env(pool: PgPool) -> Option<Self> {
        let key_file = std::env::var("VAPID_PRIVATE_KEY_FILE").ok()?;
        let subject = std::env::var("VAPID_SUBJECT")
            .unwrap_or_else(|_| "mailto:admin@glider.flights".to_string());

        let private_key_pem = match std::fs::read(&key_file) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to read VAPID private key from {}: {}", key_file, e);
                return None;
            }
        };

        info!(
            key_file = %key_file,
            "Push notification sender initialized (VAPID configured)"
        );
        Some(Self {
            pool,
            vapid_private_key: private_key_pem,
            vapid_subject: subject,
        })
    }

    /// Send a takeoff notification to all club members who have subscribed
    pub async fn send_takeoff_notification(
        &self,
        club_id: Uuid,
        club_name: &str,
        flight_id: Uuid,
        aircraft_registration: Option<&str>,
        aircraft_model: &str,
        airport_ident: Option<&str>,
    ) {
        let payload = FlightNotificationPayload {
            event_type: "takeoff".to_string(),
            flight_id,
            club_id,
            club_name: club_name.to_string(),
            aircraft_registration: aircraft_registration.map(|s| s.to_string()),
            aircraft_model: aircraft_model.to_string(),
            airport_ident: airport_ident.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.send_to_club_subscribers(club_id, "takeoff", &payload)
            .await;
    }

    /// Send a landing notification to all club members who have subscribed
    pub async fn send_landing_notification(
        &self,
        club_id: Uuid,
        club_name: &str,
        flight_id: Uuid,
        aircraft_registration: Option<&str>,
        aircraft_model: &str,
        airport_ident: Option<&str>,
    ) {
        let payload = FlightNotificationPayload {
            event_type: "landing".to_string(),
            flight_id,
            club_id,
            club_name: club_name.to_string(),
            aircraft_registration: aircraft_registration.map(|s| s.to_string()),
            aircraft_model: aircraft_model.to_string(),
            airport_ident: airport_ident.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.send_to_club_subscribers(club_id, "landing", &payload)
            .await;
    }

    async fn send_to_club_subscribers(
        &self,
        club_id: Uuid,
        event_type: &str,
        payload: &FlightNotificationPayload,
    ) {
        let repo = PushSubscriptionsRepository::new(self.pool.clone());

        let subscribers = match event_type {
            "takeoff" => repo.get_takeoff_subscribers(club_id).await,
            "landing" => repo.get_landing_subscribers(club_id).await,
            _ => return,
        };

        let subscribers = match subscribers {
            Ok(subs) => subs,
            Err(e) => {
                error!(
                    error = %e,
                    club_id = %club_id,
                    "Failed to get push subscribers"
                );
                metrics::counter!("push_notifications.failed_total").increment(1);
                return;
            }
        };

        if subscribers.is_empty() {
            return;
        }

        let payload_json = match serde_json::to_string(payload) {
            Ok(json) => json,
            Err(e) => {
                error!(error = %e, "Failed to serialize push notification payload");
                return;
            }
        };

        info!(
            club_id = %club_id,
            event_type = event_type,
            subscriber_count = subscribers.len(),
            "Sending push notifications"
        );

        let client = match IsahcWebPushClient::new() {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "Failed to create web push client");
                metrics::counter!("push_notifications.failed_total")
                    .increment(subscribers.len() as u64);
                return;
            }
        };

        for sub in &subscribers {
            let subscription_info =
                SubscriptionInfo::new(&sub.endpoint, &sub.p256dh_key, &sub.auth_key);

            let mut sig_builder = match VapidSignatureBuilder::from_pem(
                std::io::Cursor::new(&self.vapid_private_key),
                &subscription_info,
            ) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(error = %e, "Failed to build VAPID signature");
                    metrics::counter!("push_notifications.failed_total").increment(1);
                    continue;
                }
            };
            sig_builder.add_claim("sub", self.vapid_subject.clone());

            let vapid_signature = match sig_builder.build() {
                Ok(sig) => sig,
                Err(e) => {
                    error!(error = %e, "Failed to sign VAPID");
                    metrics::counter!("push_notifications.failed_total").increment(1);
                    continue;
                }
            };

            let mut builder = WebPushMessageBuilder::new(&subscription_info);
            builder.set_payload(ContentEncoding::Aes128Gcm, payload_json.as_bytes());
            builder.set_vapid_signature(vapid_signature);

            let message = match builder.build() {
                Ok(msg) => msg,
                Err(e) => {
                    error!(error = %e, "Failed to build push message");
                    metrics::counter!("push_notifications.failed_total").increment(1);
                    continue;
                }
            };

            match client.send(message).await {
                Ok(_) => {
                    metrics::counter!("push_notifications.sent_total").increment(1);
                }
                Err(e) => {
                    // Check for 410 Gone - subscription expired
                    let error_str = format!("{}", e);
                    if error_str.contains("410") || error_str.contains("Gone") {
                        warn!(
                            endpoint = %sub.endpoint,
                            "Push subscription expired (410 Gone), removing"
                        );
                        let repo_cleanup = PushSubscriptionsRepository::new(self.pool.clone());
                        let endpoint = sub.endpoint.clone();
                        tokio::spawn(async move {
                            if let Err(e) = repo_cleanup.delete_by_endpoint(&endpoint).await {
                                error!(error = %e, "Failed to clean up expired subscription");
                            }
                            metrics::counter!("push_notifications.subscriptions_cleaned_total")
                                .increment(1);
                        });
                    } else {
                        error!(
                            error = %e,
                            endpoint = %sub.endpoint,
                            "Failed to send push notification"
                        );
                    }
                    metrics::counter!("push_notifications.failed_total").increment(1);
                }
            }
        }
    }
}
