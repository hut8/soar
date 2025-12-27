use anyhow::Result;
use std::env;

/// Get the environment name for display purposes
/// Returns "Production", "Staging", or "Development"
fn get_environment_name() -> String {
    match env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "production" => "Production".to_string(),
        "staging" => "Staging".to_string(),
        _ => "Development".to_string(),
    }
}

/// Get the staging prefix for email subjects
/// Returns "[STAGING] " if SOAR_ENV=staging, empty string otherwise
fn get_staging_prefix() -> &'static str {
    match env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "staging" => "[STAGING] ",
        _ => "",
    }
}

#[derive(Debug, Clone)]
pub struct MigrationEmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub to_address: String,
}

impl MigrationEmailConfig {
    /// Load email configuration from environment variables
    /// Uses MIGRATION_ALERT_EMAIL if set, otherwise falls back to FROM_EMAIL
    pub fn from_env() -> Result<Self> {
        let from_email = env::var("FROM_EMAIL")
            .or_else(|_| env::var("EMAIL_FROM"))
            .map_err(|_| anyhow::anyhow!("FROM_EMAIL or EMAIL_FROM not set"))?;

        // Use MIGRATION_ALERT_EMAIL if set, otherwise use FROM_EMAIL as recipient
        let to_email = env::var("MIGRATION_ALERT_EMAIL").unwrap_or_else(|_| from_email.clone());

        Ok(MigrationEmailConfig {
            smtp_server: env::var("SMTP_SERVER").unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            smtp_username: env::var("SMTP_USERNAME")
                .map_err(|_| anyhow::anyhow!("SMTP_USERNAME not set"))?,
            smtp_password: env::var("SMTP_PASSWORD")
                .map_err(|_| anyhow::anyhow!("SMTP_PASSWORD not set"))?,
            from_address: from_email,
            to_address: to_email,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub success: bool,
    pub applied_migrations: Vec<String>,
    pub duration_secs: f64,
    pub error_message: Option<String>,
    pub log_excerpt: Option<String>,
}

impl MigrationReport {
    pub fn success(applied_migrations: Vec<String>, duration_secs: f64) -> Self {
        Self {
            success: true,
            applied_migrations,
            duration_secs,
            error_message: None,
            log_excerpt: None,
        }
    }

    pub fn failure(error: String, log_excerpt: Option<String>, duration_secs: f64) -> Self {
        Self {
            success: false,
            applied_migrations: Vec::new(),
            duration_secs,
            error_message: Some(error),
            log_excerpt,
        }
    }

    fn format_duration(secs: f64) -> String {
        if secs < 60.0 {
            format!("{:.1}s", secs)
        } else if secs < 3600.0 {
            format!("{:.1}m", secs / 60.0)
        } else {
            format!("{:.1}h", secs / 3600.0)
        }
    }

    pub fn to_html(&self) -> String {
        let environment = get_environment_name();
        let status_color = if self.success { "#28a745" } else { "#dc3545" };
        let status_text = if self.success {
            "✓ SUCCESS"
        } else {
            "✗ FAILED"
        };

        let hostname = env::var("HOSTNAME")
            .or_else(|_| {
                std::process::Command::new("hostname")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .ok_or(env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| "unknown".to_string());

        let mut html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 800px; margin: 0 auto; background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; margin-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; margin-bottom: 15px; border-bottom: 2px solid #007bff; padding-bottom: 5px; }}
        .status {{ font-size: 24px; font-weight: bold; color: {}; margin-bottom: 20px; }}
        .summary {{ background-color: #f8f9fa; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .error-box {{ background-color: #f8d7da; border: 1px solid #f5c6cb; padding: 10px; border-radius: 4px; margin-top: 15px; color: #721c24; }}
        .log-box {{ background-color: #f5f5f5; border: 1px solid #ddd; padding: 10px; border-radius: 4px; margin-top: 15px; font-family: monospace; font-size: 12px; white-space: pre-wrap; word-wrap: break-word; }}
        .migration-list {{ background-color: #d4edda; border: 1px solid #c3e6cb; padding: 10px; border-radius: 4px; margin-top: 15px; color: #155724; }}
        .migration-list ul {{ margin: 5px 0; padding-left: 20px; }}
        .footer {{ margin-top: 20px; color: #666; font-size: 12px; text-align: center; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>SOAR Database Migration Report - {}</h1>
        <div class="status">{}</div>
        <div class="summary">
            <strong>Environment:</strong> {}<br>
            <strong>Hostname:</strong> {}<br>
            <strong>Duration:</strong> {}<br>
            <strong>Time:</strong> {}
        </div>"#,
            status_color,
            environment,
            status_text,
            environment,
            hostname,
            Self::format_duration(self.duration_secs),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );

        if self.success {
            if self.applied_migrations.is_empty() {
                html.push_str(
                    r#"
        <div class="migration-list">
            <strong>No pending migrations to apply</strong><br>
            Database schema is up to date.
        </div>"#,
                );
            } else {
                html.push_str(&format!(
                    r#"
        <div class="migration-list">
            <strong>Applied {} migration(s):</strong>
            <ul>"#,
                    self.applied_migrations.len()
                ));
                for migration in &self.applied_migrations {
                    html.push_str(&format!("                <li>{}</li>\n", migration));
                }
                html.push_str(
                    r#"            </ul>
        </div>"#,
                );
            }
        } else {
            if let Some(error) = &self.error_message {
                html.push_str(&format!(
                    r#"
        <div class="error-box">
            <strong>Error:</strong><br>
            {}
        </div>"#,
                    error
                ));
            }

            if let Some(log) = &self.log_excerpt {
                html.push_str(&format!(
                    r#"
        <h2>Migration Output</h2>
        <div class="log-box">{}</div>"#,
                    log
                ));
            }
        }

        html.push_str(
            r#"
        <div class="footer">
            Generated by SOAR Migration System
        </div>
    </div>
</body>
</html>"#,
        );

        html
    }
}

pub fn send_migration_email_report(
    config: &MigrationEmailConfig,
    report: &MigrationReport,
) -> Result<()> {
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::transport::smtp::client::TlsParametersBuilder;
    use lettre::{Message, SmtpTransport, Transport};
    use std::time::Duration;
    use tracing::{info, warn};

    let staging_prefix = get_staging_prefix();
    let subject = if report.success {
        format!(
            "{}✓ SOAR Database Migration COMPLETED - {}",
            staging_prefix,
            chrono::Local::now().format("%Y-%m-%d")
        )
    } else {
        format!(
            "{}✗ SOAR Database Migration FAILED - {}",
            staging_prefix,
            chrono::Local::now().format("%Y-%m-%d")
        )
    };

    let html_body = report.to_html();

    info!(
        "Sending migration email report to {} (success: {})",
        config.to_address, report.success
    );

    let email = Message::builder()
        .from(config.from_address.parse()?)
        .to(config.to_address.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body)?;

    let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

    // Configure SMTP transport based on port:
    // - Port 1025: Insecure (Mailpit for local testing)
    // - Port 465: Implicit TLS (TLS wrapper - immediate TLS connection)
    // - Port 587: STARTTLS (start plain, upgrade to TLS)
    let mailer = if config.smtp_port == 1025 {
        // Use builder for insecure local SMTP (Mailpit)
        info!("Using insecure SMTP connection for port 1025 (Mailpit) without TLS");
        SmtpTransport::builder_dangerous(&config.smtp_server)
            .port(config.smtp_port)
            .tls(lettre::transport::smtp::client::Tls::None)
            .timeout(Some(Duration::from_secs(30)))
            .build()
    } else if config.smtp_port == 465 {
        // Port 465 uses implicit TLS (TLS wrapper - SMTPS)
        info!("Using implicit TLS (SMTPS) for port 465");
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
        // Port 587 and others use STARTTLS
        info!("Using STARTTLS for port {}", config.smtp_port);
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

    match mailer.send(&email) {
        Ok(_) => {
            info!("Migration email report sent successfully");
            Ok(())
        }
        Err(e) => {
            warn!("Failed to send migration email report: {}", e);
            Err(anyhow::anyhow!("Failed to send email: {}", e))
        }
    }
}
