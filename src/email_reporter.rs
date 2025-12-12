use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::time::Duration;
use tracing::{info, warn};

use crate::aircraft::AircraftModel;

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub to_address: String,
}

impl EmailConfig {
    /// Load email configuration from environment variables
    /// Reads from ~/.localrc if sourced into environment
    pub fn from_env() -> Result<Self> {
        Ok(EmailConfig {
            smtp_server: std::env::var("SMTP_SERVER")
                .unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            smtp_port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            smtp_username: std::env::var("SMTP_USERNAME")
                .map_err(|_| anyhow::anyhow!("SMTP_USERNAME not set"))?,
            smtp_password: std::env::var("SMTP_PASSWORD")
                .map_err(|_| anyhow::anyhow!("SMTP_PASSWORD not set"))?,
            from_address: std::env::var("FROM_EMAIL")
                .or_else(|_| std::env::var("EMAIL_FROM"))
                .map_err(|_| anyhow::anyhow!("FROM_EMAIL or EMAIL_FROM not set"))?,
            to_address: std::env::var("EMAIL_TO")
                .or_else(|_| std::env::var("TO_EMAIL"))
                .map_err(|_| anyhow::anyhow!("EMAIL_TO or TO_EMAIL not set"))?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EntityMetrics {
    pub name: String,
    pub duration_secs: f64,
    pub records_loaded: usize,
    pub records_in_db: Option<i64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub failed_items: Option<Vec<String>>, // For tracking specific items that failed (e.g., receiver callsigns)
}

impl EntityMetrics {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            duration_secs: 0.0,
            records_loaded: 0,
            records_in_db: None,
            success: true,
            error_message: None,
            failed_items: None,
        }
    }

    pub fn with_error(name: &str, error: String) -> Self {
        Self {
            name: name.to_string(),
            duration_secs: 0.0,
            records_loaded: 0,
            records_in_db: None,
            success: false,
            error_message: Some(error),
            failed_items: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataLoadReport {
    pub total_duration_secs: f64,
    pub entities: Vec<EntityMetrics>,
    pub overall_success: bool,
    pub duplicate_devices: Vec<AircraftModel>,
}

impl Default for DataLoadReport {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLoadReport {
    pub fn new() -> Self {
        Self {
            total_duration_secs: 0.0,
            entities: Vec::new(),
            overall_success: true,
            duplicate_devices: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, metrics: EntityMetrics) {
        if !metrics.success {
            self.overall_success = false;
        }
        self.entities.push(metrics);
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
        let status_color = if self.overall_success {
            "#28a745"
        } else {
            "#dc3545"
        };
        let status_text = if self.overall_success {
            "✓ SUCCESS"
        } else {
            "✗ FAILED"
        };

        let mut html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 800px; margin: 0 auto; background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; margin-bottom: 10px; }}
        .status {{ font-size: 24px; font-weight: bold; color: {}; margin-bottom: 20px; }}
        .summary {{ background-color: #f8f9fa; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #007bff; color: white; font-weight: bold; }}
        tr:hover {{ background-color: #f5f5f5; }}
        .success {{ color: #28a745; }}
        .failure {{ color: #dc3545; }}
        .error-box {{ background-color: #f8d7da; border: 1px solid #f5c6cb; padding: 10px; border-radius: 4px; margin-top: 5px; color: #721c24; }}
        .footer {{ margin-top: 20px; color: #666; font-size: 12px; text-align: center; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>SOAR Data Load Report</h1>
        <div class="status">{}</div>
        <div class="summary">
            <strong>Total Duration:</strong> {}<br>
            <strong>Entities Processed:</strong> {}<br>
            <strong>Time:</strong> {}
        </div>
        <table>
            <tr>
                <th>Entity</th>
                <th>Status</th>
                <th>Duration</th>
                <th>Records Loaded</th>
                <th>Total in DB</th>
            </tr>"#,
            status_color,
            status_text,
            Self::format_duration(self.total_duration_secs),
            self.entities.len(),
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );

        for entity in &self.entities {
            let status_class = if entity.success { "success" } else { "failure" };
            let status_symbol = if entity.success { "✓" } else { "✗" };
            let db_count = entity
                .records_in_db
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_string());

            html.push_str(&format!(
                r#"
            <tr>
                <td>{}</td>
                <td class="{}">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#,
                entity.name,
                status_class,
                status_symbol,
                Self::format_duration(entity.duration_secs),
                entity.records_loaded,
                db_count
            ));

            if let Some(error) = &entity.error_message {
                html.push_str(&format!(
                    r#"
            <tr>
                <td colspan="5">
                    <div class="error-box">
                        <strong>Error:</strong> {}
                    </div>
                </td>
            </tr>"#,
                    error
                ));
            }

            if let Some(failed_items) = &entity.failed_items
                && !failed_items.is_empty()
            {
                // Parse failed items which may contain lat/lon coordinates in format: "callsign|lat,lon"
                let mut items_html = String::new();
                for (i, item) in failed_items.iter().enumerate() {
                    if i > 0 {
                        items_html.push_str(", ");
                    }

                    // Check if item contains coordinates (format: "callsign|lat,lon")
                    if let Some((callsign, coords)) = item.split_once('|') {
                        // Create a Google Maps link for the coordinates
                        let maps_url =
                            format!("https://www.google.com/maps/search/?api=1&query={}", coords);
                        items_html.push_str(&format!(
                            r#"{} (<a href="{}" target="_blank" style="color: #721c24; text-decoration: underline;">{}</a>)"#,
                            callsign, maps_url, coords
                        ));
                    } else {
                        // No coordinates, just display the item as-is
                        items_html.push_str(item);
                    }
                }

                html.push_str(&format!(
                    r#"
            <tr>
                <td colspan="5">
                    <div class="error-box">
                        <strong>Failed Items ({}):</strong> {}
                    </div>
                </td>
            </tr>"#,
                    failed_items.len(),
                    items_html
                ));
            }
        }

        html.push_str(
            r#"
        </table>"#,
        );

        // Add duplicate devices table if any exist
        if !self.duplicate_devices.is_empty() {
            html.push_str(
                r#"
        <h2 style="margin-top: 30px; color: #dc3545;">⚠ Duplicate Aircraft Addresses</h2>
        <p style="color: #666; margin-bottom: 15px;">
            The following addresses appear multiple times with different address types:
        </p>
        <table>
            <tr>
                <th>Address (Hex)</th>
                <th>Address Type</th>
                <th>Registration</th>
                <th>Aircraft Model</th>
                <th>From DDB</th>
                <th>Tracked</th>
                <th>Last Fix</th>
            </tr>"#,
            );

            for device in &self.duplicate_devices {
                let address_hex = format!("{:06X}", device.address);
                let last_fix = device
                    .last_fix_at
                    .map(|ts| ts.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "-".to_string());

                html.push_str(&format!(
                    r#"
            <tr>
                <td><strong>{}</strong></td>
                <td>{:?}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#,
                    address_hex,
                    device.address_type,
                    device.registration,
                    device.aircraft_model,
                    if device.from_ddb { "Yes" } else { "No" },
                    if device.tracked { "Yes" } else { "No" },
                    last_fix
                ));
            }

            html.push_str(
                r#"
        </table>"#,
            );
        }

        html.push_str(
            r#"
        <div class="footer">
            Generated by SOAR Data Loader
        </div>
    </div>
</body>
</html>"#,
        );

        html
    }
}

pub fn send_email_report(config: &EmailConfig, report: &DataLoadReport) -> Result<()> {
    let subject = if report.overall_success {
        format!(
            "✓ SOAR Data Load Complete - {}",
            chrono::Local::now().format("%Y-%m-%d")
        )
    } else {
        format!(
            "✗ SOAR Data Load FAILED - {}",
            chrono::Local::now().format("%Y-%m-%d")
        )
    };

    let html_body = report.to_html();

    info!(
        "Sending email report to {} (success: {})",
        config.to_address, report.overall_success
    );

    let email = Message::builder()
        .from(config.from_address.parse()?)
        .to(config.to_address.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body)?;

    let creds = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

    let mailer = SmtpTransport::relay(&config.smtp_server)?
        .port(config.smtp_port)
        .credentials(creds)
        .timeout(Some(Duration::from_secs(30)))
        .build();

    match mailer.send(&email) {
        Ok(_) => {
            info!("Email report sent successfully");
            Ok(())
        }
        Err(e) => {
            warn!("Failed to send email report: {}", e);
            Err(anyhow::anyhow!("Failed to send email: {}", e))
        }
    }
}

pub fn send_failure_email(config: &EmailConfig, entity: &str, error: &str) -> Result<()> {
    let mut report = DataLoadReport::new();
    report.add_entity(EntityMetrics::with_error(entity, error.to_string()));
    send_email_report(config, &report)
}
