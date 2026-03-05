use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::env;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::schema::{clubs, users};

type PgPool = Pool<ConnectionManager<PgConnection>>;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub from_name: String,
}

impl SmtpConfig {
    /// Load SMTP configuration from environment variables.
    /// Returns None if required env vars are not set (email notifications disabled).
    pub fn from_env() -> Option<Self> {
        let smtp_username = env::var("SMTP_USERNAME").ok()?;
        let smtp_password = env::var("SMTP_PASSWORD").ok()?;
        let from_address = env::var("FROM_EMAIL").ok()?;

        Some(SmtpConfig {
            smtp_server: env::var("SMTP_SERVER").unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            smtp_username,
            smtp_password,
            from_address,
            from_name: env::var("FROM_NAME").unwrap_or_else(|_| "SOAR".to_string()),
        })
    }
}

/// Build an SMTP transport based on the port configuration.
fn build_smtp_transport(config: &SmtpConfig) -> Result<lettre::SmtpTransport> {
    use lettre::SmtpTransport;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::transport::smtp::client::TlsParametersBuilder;

    let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

    let mailer = if config.smtp_port == 1025 {
        // Insecure local SMTP (Mailpit)
        SmtpTransport::builder_dangerous(&config.smtp_server)
            .port(config.smtp_port)
            .tls(lettre::transport::smtp::client::Tls::None)
            .timeout(Some(Duration::from_secs(30)))
            .build()
    } else if config.smtp_port == 465 {
        // Implicit TLS (SMTPS)
        let tls_params = TlsParametersBuilder::new(config.smtp_server.clone())
            .dangerous_accept_invalid_certs(true)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create TLS parameters: {}", e))?;
        SmtpTransport::relay(&config.smtp_server)?
            .port(config.smtp_port)
            .credentials(creds)
            .tls(lettre::transport::smtp::client::Tls::Wrapper(tls_params))
            .timeout(Some(Duration::from_secs(30)))
            .build()
    } else {
        // STARTTLS (port 587 and others)
        let tls_params = TlsParametersBuilder::new(config.smtp_server.clone())
            .dangerous_accept_invalid_certs(true)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create TLS parameters: {}", e))?;
        SmtpTransport::relay(&config.smtp_server)?
            .port(config.smtp_port)
            .credentials(creds)
            .tls(lettre::transport::smtp::client::Tls::Required(tls_params))
            .timeout(Some(Duration::from_secs(30)))
            .build()
    };

    Ok(mailer)
}

/// Send an email using the configured SMTP transport.
fn send_email(config: &SmtpConfig, to: &str, subject: &str, html_body: String) -> Result<()> {
    use lettre::message::header::ContentType;
    use lettre::{Message, Transport};

    let from = format!("{} <{}>", config.from_name, config.from_address);

    let email = Message::builder()
        .from(from.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body)?;

    let mailer = build_smtp_transport(config)?;
    mailer.send(&email)?;
    Ok(())
}

/// Staging prefix for email subjects
fn staging_prefix() -> &'static str {
    match env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "staging" => "[STAGING] ",
        _ => "",
    }
}

/// Send email notifications to club admins when a new join request is created.
///
/// This is designed to be called from a `tokio::spawn` — it logs errors
/// rather than propagating them.
pub async fn send_join_request_notification(
    pool: &PgPool,
    club_id: Uuid,
    requester_name: &str,
) -> Result<()> {
    let config = match SmtpConfig::from_env() {
        Some(c) => c,
        None => {
            info!("SMTP not configured, skipping join request notification");
            return Ok(());
        }
    };

    let pool = pool.clone();
    let requester_name = requester_name.to_string();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut conn = pool.get()?;

        // Get club name
        let club_name: String = clubs::table
            .filter(clubs::id.eq(club_id))
            .select(clubs::name)
            .first(&mut conn)?;

        // Get club admin emails
        let admin_emails: Vec<String> = users::table
            .filter(users::club_id.eq(club_id))
            .filter(users::is_club_admin.eq(true))
            .filter(users::email.is_not_null())
            .filter(users::deleted_at.is_null())
            .select(users::email.assume_not_null())
            .load(&mut conn)?;

        if admin_emails.is_empty() {
            warn!(
                club_id = %club_id,
                "No club admins with email addresses found for join request notification"
            );
            return Ok(());
        }

        let prefix = staging_prefix();
        let subject = format!(
            "{}New Join Request for {}",
            prefix, club_name
        );

        let html_body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 600px; margin: 0 auto; background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; font-size: 20px; }}
        .message {{ margin: 15px 0; color: #555; }}
        .footer {{ margin-top: 20px; color: #999; font-size: 12px; text-align: center; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>New Join Request</h1>
        <div class="message">
            <p><strong>{requester}</strong> has requested to join <strong>{club}</strong>.</p>
            <p>Please log in to SOAR to review and approve or reject this request.</p>
        </div>
        <div class="footer">
            SOAR - Soaring Observation And Records
        </div>
    </div>
</body>
</html>"#,
            requester = html_escape(&requester_name),
            club = html_escape(&club_name),
        );

        for email_addr in &admin_emails {
            match send_email(&config, email_addr, &subject, html_body.clone()) {
                Ok(()) => {
                    info!(
                        to = %email_addr,
                        club = %club_name,
                        "Sent join request notification email"
                    );
                }
                Err(e) => {
                    error!(
                        to = %email_addr,
                        error = %e,
                        "Failed to send join request notification email"
                    );
                }
            }
        }

        Ok(())
    })
    .await?
}
