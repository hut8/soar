use anyhow::Result;
use chrono::{DateTime, Utc};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Mailbox, Message, MultiPart, SinglePart, header::ContentType},
    transport::smtp::{
        authentication::Credentials, client::TlsParametersBuilder, response::Response,
    },
};

/// Get the environment name for display purposes
/// Returns "Production", "Staging", or "Development"
fn get_environment_name() -> String {
    match std::env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "production" => "Production".to_string(),
        "staging" => "Staging".to_string(),
        _ => "Development".to_string(),
    }
}

/// Data about an aircraft for email display
#[derive(Debug, Clone)]
pub struct AircraftEmailData {
    /// Aircraft ID (UUID)
    pub id: uuid::Uuid,
    /// Aircraft registration (e.g., "N8437D")
    pub registration: Option<String>,
    /// Aircraft model (e.g., "Piper Pacer")
    pub aircraft_model: String,
    /// Hex address (6-digit, e.g., "0513AD")
    pub hex_address: String,
}

impl AircraftEmailData {
    /// Get display name for the aircraft
    /// Priority: Model + Registration > Registration only > Model only > Hex address
    pub fn display_name(&self) -> String {
        let has_registration = self.registration.as_ref().is_some_and(|r| !r.is_empty());
        let has_model = !self.aircraft_model.is_empty();

        match (has_model, has_registration) {
            (true, true) => format!(
                "{} {}",
                self.aircraft_model,
                self.registration.as_ref().unwrap()
            ),
            (false, true) => self.registration.clone().unwrap(),
            (true, false) => self.aircraft_model.clone(),
            (false, false) => self.hex_address.clone(),
        }
    }

    /// Generate filename component for the aircraft
    /// Returns "-REGISTRATION" if available, empty string otherwise
    pub fn filename_component(&self) -> String {
        self.registration
            .as_ref()
            .filter(|r| !r.is_empty())
            .map(|r| format!("-{}", r))
            .unwrap_or_default()
    }
}

/// Data about a flight for email display
#[derive(Debug, Clone)]
pub struct FlightEmailData {
    /// Flight ID
    pub flight_id: uuid::Uuid,
    /// Aircraft information
    pub aircraft: AircraftEmailData,
    /// Takeoff time (None if detected airborne)
    pub takeoff_time: Option<DateTime<Utc>>,
    /// Landing time
    pub landing_time: Option<DateTime<Utc>>,
    /// Departure airport identifier (e.g., "KLVK")
    pub departure_airport: Option<String>,
    /// Departure airport name (e.g., "Livermore Municipal")
    pub departure_airport_name: Option<String>,
    /// Arrival airport identifier
    pub arrival_airport: Option<String>,
    /// Arrival airport name
    pub arrival_airport_name: Option<String>,
    /// Flight duration in seconds
    pub duration_seconds: Option<i64>,
    /// Total distance in meters
    pub total_distance_meters: Option<f64>,
    /// Maximum displacement from departure in meters
    pub max_displacement_meters: Option<f64>,
    /// Takeoff runway identifier
    pub takeoff_runway: Option<String>,
    /// Landing runway identifier
    pub landing_runway: Option<String>,
    /// Whether flight was detected airborne (no true takeoff time)
    pub detected_airborne: bool,
    /// Whether flight timed out (signal lost)
    pub timed_out: bool,
}

/// Get the staging prefix for email subjects
/// Returns "[STAGING] " if SOAR_ENV=staging, empty string otherwise
fn get_staging_prefix() -> &'static str {
    match std::env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "staging" => "[STAGING] ",
        _ => "",
    }
}

/// Create a properly formatted Mailbox with display name
/// This handles special characters in display names by using lettre's Mailbox type
fn create_mailbox(name: &str, email: &str) -> Result<Mailbox> {
    let address = email.parse()?;
    Ok(Mailbox::new(Some(name.to_string()), address))
}

pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
    from_name: String,
}

impl EmailService {
    pub fn new() -> Result<Self> {
        let smtp_server = std::env::var("SMTP_SERVER")
            .map_err(|_| anyhow::anyhow!("SMTP_SERVER environment variable not set"))?;

        let smtp_port: u16 = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid SMTP_PORT"))?;

        let smtp_username = std::env::var("SMTP_USERNAME")
            .map_err(|_| anyhow::anyhow!("SMTP_USERNAME environment variable not set"))?;

        let smtp_password = std::env::var("SMTP_PASSWORD")
            .map_err(|_| anyhow::anyhow!("SMTP_PASSWORD environment variable not set"))?;

        let from_email = std::env::var("FROM_EMAIL")
            .map_err(|_| anyhow::anyhow!("FROM_EMAIL environment variable not set"))?;

        let from_name = std::env::var("FROM_NAME").unwrap_or_else(|_| "SOAR".to_string());

        let creds = Credentials::new(smtp_username, smtp_password);

        // Configure SMTP transport based on port:
        // - Port 1025: Insecure (Mailpit for local testing)
        // - Port 465: Implicit TLS (TLS wrapper - immediate TLS connection)
        // - Port 587: STARTTLS (start plain, upgrade to TLS)
        let mailer = if smtp_port == 1025 {
            // Use builder for insecure local SMTP (Mailpit)
            // Mailpit doesn't support TLS, so we need to disable it completely
            tracing::info!("Using insecure SMTP connection for port 1025 (Mailpit) without TLS");
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_server)
                .port(smtp_port)
                .tls(lettre::transport::smtp::client::Tls::None)
                .build()
        } else if smtp_port == 465 {
            // Port 465 uses implicit TLS (TLS wrapper - SMTPS)
            // Connection starts with TLS immediately, no STARTTLS upgrade
            tracing::info!("Using implicit TLS (SMTPS) for port 465");
            let tls_params = TlsParametersBuilder::new(smtp_server.clone())
                .dangerous_accept_invalid_certs(true)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create TLS parameters: {}", e))?;
            AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_server)?
                .port(smtp_port)
                .credentials(creds)
                .tls(lettre::transport::smtp::client::Tls::Wrapper(tls_params))
                .build()
        } else {
            // Port 587 and others use STARTTLS
            // Connection starts plain and upgrades to TLS
            tracing::info!("Using STARTTLS for port {}", smtp_port);
            let tls_params = TlsParametersBuilder::new(smtp_server.clone())
                .dangerous_accept_invalid_certs(true)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create TLS parameters: {}", e))?;
            AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_server)?
                .port(smtp_port)
                .credentials(creds)
                .tls(lettre::transport::smtp::client::Tls::Required(tls_params))
                .build()
        };

        Ok(Self {
            mailer,
            from_email,
            from_name,
        })
    }

    pub async fn send_password_reset_email(
        &self,
        to_email: &str,
        to_name: &str,
        reset_token: &str,
    ) -> Result<Response> {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let reset_url = format!("{}/reset-password?token={}", base_url, reset_token);

        let subject = format!("{}Password Reset Request - SOAR", get_staging_prefix());
        let body = format!(
            r#"Hello {},

We received a request to reset your password for your SOAR account.

To reset your password, please click the following link:
{}

This link will expire in 1 hour for security reasons.

If you did not request a password reset, please ignore this email and your password will remain unchanged.

Best regards,
The SOAR Team"#,
            to_name, reset_url
        );

        let email = Message::builder()
            .from(create_mailbox(&self.from_name, &self.from_email)?)
            .to(create_mailbox(to_name, to_email)?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        let response = self.mailer.send(email).await?;
        Ok(response)
    }

    pub async fn send_email_verification(
        &self,
        to_email: &str,
        to_name: &str,
        verification_token: &str,
    ) -> Result<Response> {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let verification_url = format!("{}/verify-email?token={}", base_url, verification_token);

        let subject = format!("{}Verify Your Email Address - SOAR", get_staging_prefix());
        let body = format!(
            r#"Hello {},

Thank you for registering with SOAR! To complete your account setup, please verify your email address.

Click the following link to verify your email:
{}

This link will expire in 24 hours for security reasons.

If you did not create an account with SOAR, please ignore this email.

Best regards,
The SOAR Team"#,
            to_name, verification_url
        );

        let email = Message::builder()
            .from(create_mailbox(&self.from_name, &self.from_email)?)
            .to(create_mailbox(to_name, to_email)?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        let response = self.mailer.send(email).await?;
        Ok(response)
    }

    /// Send pilot invitation email
    /// This is sent to pilots who have been added to the club roster but don't have login access yet
    pub async fn send_pilot_invitation_email(
        &self,
        to_email: &str,
        to_name: &str,
        verification_token: &str,
    ) -> Result<Response> {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let registration_url = format!(
            "{}/complete-registration?token={}",
            base_url, verification_token
        );

        let subject = format!(
            "{}You've been invited to join SOAR - Complete Your Registration",
            get_staging_prefix()
        );
        let body = format!(
            r#"Hello {},

You've been added to your club's roster on SOAR! To access your account and manage your flight information, please complete your registration by setting a password.

Click the following link to complete your registration:
{}

This link will expire in 72 hours for security reasons.

Once you've set your password, you'll be able to:
- View your flight history
- Track your progress and achievements
- Receive flight notifications
- Access club information

If you believe you received this email in error, please ignore it or contact your club administrator.

Best regards,
The SOAR Team"#,
            to_name, registration_url
        );

        let email = Message::builder()
            .from(create_mailbox(&self.from_name, &self.from_email)?)
            .to(create_mailbox(to_name, to_email)?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        let response = self.mailer.send(email).await?;
        Ok(response)
    }

    /// Send flight completion notification with KML and IGC attachments
    /// Now sends an attractive HTML email with flight details similar to the flight details page
    #[allow(clippy::too_many_arguments)]
    pub async fn send_flight_completion_email(
        &self,
        to_email: &str,
        to_name: &str,
        flight_data: &FlightEmailData,
        kml_content: String,
        kml_filename: &str,
        igc_content: String,
        igc_filename: &str,
    ) -> Result<Response> {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let flight_url = format!("{}/flights/{}", base_url, flight_data.flight_id);
        let aircraft_url = format!("{}/aircraft/{}", base_url, flight_data.aircraft.id);
        let watchlist_url = format!("{}/watchlist", base_url);

        let aircraft_name = flight_data.aircraft.display_name();
        let environment = get_environment_name();

        let subject = format!(
            "{}Flight Completed - {}",
            get_staging_prefix(),
            aircraft_name
        );

        // Build HTML email
        let html_body = self.build_flight_html(
            to_name,
            flight_data,
            &flight_url,
            &aircraft_url,
            &watchlist_url,
            &environment,
        );

        // Build plain text fallback
        let text_body = self.build_flight_text(
            to_name,
            flight_data,
            &flight_url,
            &aircraft_url,
            &watchlist_url,
        );

        // Create attachments
        use lettre::message::Attachment;

        let kml_part = Attachment::new(kml_filename.to_string()).body(
            kml_content,
            ContentType::parse("application/vnd.google-earth.kml+xml")?,
        );

        let igc_part = Attachment::new(igc_filename.to_string())
            .body(igc_content, ContentType::parse("application/x-igc")?);

        let email = Message::builder()
            .from(create_mailbox(&self.from_name, &self.from_email)?)
            .to(create_mailbox(to_name, to_email)?)
            .subject(subject)
            .multipart(
                MultiPart::mixed()
                    .multipart(
                        MultiPart::alternative()
                            .singlepart(SinglePart::plain(text_body))
                            .singlepart(SinglePart::html(html_body)),
                    )
                    .singlepart(kml_part)
                    .singlepart(igc_part),
            )?;

        let response = self.mailer.send(email).await?;
        Ok(response)
    }

    /// Build HTML body for flight completion email
    fn build_flight_html(
        &self,
        to_name: &str,
        flight_data: &FlightEmailData,
        flight_url: &str,
        aircraft_url: &str,
        watchlist_url: &str,
        environment: &str,
    ) -> String {
        let aircraft_name = flight_data.aircraft.display_name();

        // Format takeoff time
        let takeoff_display = flight_data
            .takeoff_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Detected airborne".to_string());

        // Format landing time
        let landing_display = flight_data
            .landing_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| {
                if flight_data.timed_out {
                    "Signal lost".to_string()
                } else {
                    "In progress".to_string()
                }
            });

        // Format duration
        let duration_display = flight_data.duration_seconds.map(format_duration);

        // Format distance (convert meters to nm and km)
        let distance_display = flight_data.total_distance_meters.map(format_distance);

        // Format displacement
        let displacement_display = flight_data.max_displacement_meters.map(format_distance);

        // Build departure info
        let departure_display = match (
            &flight_data.departure_airport,
            &flight_data.departure_airport_name,
        ) {
            (Some(ident), Some(name)) => format!("{} ({})", name, ident),
            (Some(ident), None) => ident.clone(),
            _ => "Unknown".to_string(),
        };

        // Build arrival info
        let arrival_display = match (
            &flight_data.arrival_airport,
            &flight_data.arrival_airport_name,
        ) {
            (Some(ident), Some(name)) => format!("{} ({})", name, ident),
            (Some(ident), None) => ident.clone(),
            _ => "Unknown".to_string(),
        };

        // Status badge color
        let (status_text, status_color) = if flight_data.timed_out {
            ("Signal Lost", "#dc3545") // Red
        } else if flight_data.detected_airborne {
            ("Completed (Detected Airborne)", "#ffc107") // Yellow
        } else {
            ("Completed", "#28a745") // Green
        };

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; background-color: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); overflow: hidden; }}
        .header {{ background: linear-gradient(135deg, #1e3a5f 0%, #2c5282 100%); color: white; padding: 24px; text-align: center; }}
        .header h1 {{ margin: 0 0 8px 0; font-size: 24px; font-weight: 600; }}
        .header .aircraft-link {{ color: #90cdf4; text-decoration: none; font-size: 20px; font-weight: 500; }}
        .header .aircraft-link:hover {{ text-decoration: underline; }}
        .status-badge {{ display: inline-block; padding: 4px 12px; border-radius: 12px; font-size: 12px; font-weight: 600; color: white; background-color: {status_color}; margin-top: 8px; }}
        .content {{ padding: 24px; }}
        .greeting {{ font-size: 16px; margin-bottom: 16px; }}
        .stats-grid {{ display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px; margin-bottom: 24px; }}
        .stat-card {{ background-color: #f8f9fa; border-radius: 8px; padding: 16px; border-left: 4px solid #2c5282; }}
        .stat-label {{ font-size: 12px; color: #666; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px; }}
        .stat-value {{ font-size: 18px; font-weight: 600; color: #1e3a5f; }}
        .stat-subvalue {{ font-size: 12px; color: #888; margin-top: 2px; }}
        .flight-details {{ margin-bottom: 24px; }}
        .detail-row {{ display: flex; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid #eee; }}
        .detail-row:last-child {{ border-bottom: none; }}
        .detail-label {{ color: #666; }}
        .detail-value {{ font-weight: 500; text-align: right; }}
        .attachments {{ background-color: #e8f4fd; border-radius: 8px; padding: 16px; margin-bottom: 24px; }}
        .attachments h3 {{ margin: 0 0 12px 0; font-size: 14px; color: #2c5282; }}
        .attachment-list {{ margin: 0; padding: 0; list-style: none; }}
        .attachment-list li {{ padding: 8px 0; border-bottom: 1px solid #cce4f5; }}
        .attachment-list li:last-child {{ border-bottom: none; }}
        .attachment-list li strong {{ color: #1e3a5f; }}
        .cta-button {{ display: inline-block; background: linear-gradient(135deg, #2c5282 0%, #1e3a5f 100%); color: white; text-decoration: none; padding: 12px 24px; border-radius: 6px; font-weight: 600; margin-right: 8px; margin-bottom: 8px; }}
        .cta-button:hover {{ opacity: 0.9; }}
        .cta-secondary {{ background: white; color: #2c5282; border: 2px solid #2c5282; }}
        .footer {{ background-color: #f8f9fa; padding: 16px 24px; text-align: center; font-size: 12px; color: #666; }}
        .footer a {{ color: #2c5282; text-decoration: none; }}
        .footer a:hover {{ text-decoration: underline; }}
        @media (max-width: 480px) {{
            .stats-grid {{ grid-template-columns: 1fr; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Flight Completed</h1>
            <a href="{aircraft_url}" class="aircraft-link">{aircraft_name}</a>
            <div class="status-badge">{status_text}</div>
        </div>
        <div class="content">
            <p class="greeting">Hello {to_name},</p>
            <p>An aircraft on your watchlist has completed a flight!</p>

            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-label">Takeoff</div>
                    <div class="stat-value">{takeoff_display}</div>
                    {takeoff_runway_html}
                </div>
                <div class="stat-card">
                    <div class="stat-label">Landing</div>
                    <div class="stat-value">{landing_display}</div>
                    {landing_runway_html}
                </div>
                {duration_html}
                {distance_html}
            </div>

            <div class="flight-details">
                <div class="detail-row">
                    <span class="detail-label">Departure</span>
                    <span class="detail-value">{departure_display}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Arrival</span>
                    <span class="detail-value">{arrival_display}</span>
                </div>
                {displacement_html}
                <div class="detail-row">
                    <span class="detail-label">Aircraft Address</span>
                    <span class="detail-value">{hex_address}</span>
                </div>
            </div>

            <div class="attachments">
                <h3>📎 Attached Files</h3>
                <ul class="attachment-list">
                    <li><strong>KML File</strong> - Open in Google Earth or other mapping applications</li>
                    <li><strong>IGC File</strong> - Standard flight log format for analysis software</li>
                </ul>
            </div>

            <div style="text-align: center; margin-bottom: 16px;">
                <a href="{flight_url}" class="cta-button">View Flight Details</a>
                <a href="{watchlist_url}" class="cta-button cta-secondary">Manage Watchlist</a>
            </div>
        </div>
        <div class="footer">
            <p>You received this email because you're watching this aircraft on SOAR.</p>
            <p><a href="{watchlist_url}">Manage email preferences</a> | Environment: {environment}</p>
            <p style="margin-top: 12px; color: #999;">Generated by SOAR Flight Tracker</p>
        </div>
    </div>
</body>
</html>"#,
            status_color = status_color,
            aircraft_url = aircraft_url,
            aircraft_name = html_escape(&aircraft_name),
            status_text = status_text,
            to_name = html_escape(to_name),
            takeoff_display = html_escape(&takeoff_display),
            landing_display = html_escape(&landing_display),
            takeoff_runway_html = flight_data
                .takeoff_runway
                .as_ref()
                .map(|r| format!(
                    r#"<div class="stat-subvalue">Runway {}</div>"#,
                    html_escape(r)
                ))
                .unwrap_or_default(),
            landing_runway_html = flight_data
                .landing_runway
                .as_ref()
                .map(|r| format!(
                    r#"<div class="stat-subvalue">Runway {}</div>"#,
                    html_escape(r)
                ))
                .unwrap_or_default(),
            duration_html = duration_display
                .map(|d| format!(
                    r#"<div class="stat-card">
                    <div class="stat-label">Duration</div>
                    <div class="stat-value">{}</div>
                </div>"#,
                    html_escape(&d)
                ))
                .unwrap_or_default(),
            distance_html = distance_display
                .map(|d| format!(
                    r#"<div class="stat-card">
                    <div class="stat-label">Total Distance</div>
                    <div class="stat-value">{}</div>
                </div>"#,
                    html_escape(&d)
                ))
                .unwrap_or_default(),
            departure_display = html_escape(&departure_display),
            arrival_display = html_escape(&arrival_display),
            displacement_html = displacement_display
                .map(|d| format!(
                    r#"<div class="detail-row">
                    <span class="detail-label">Max Displacement</span>
                    <span class="detail-value">{}</span>
                </div>"#,
                    html_escape(&d)
                ))
                .unwrap_or_default(),
            hex_address = html_escape(&flight_data.aircraft.hex_address),
            flight_url = flight_url,
            watchlist_url = watchlist_url,
            environment = html_escape(environment),
        )
    }

    /// Build plain text body for flight completion email
    fn build_flight_text(
        &self,
        to_name: &str,
        flight_data: &FlightEmailData,
        flight_url: &str,
        aircraft_url: &str,
        watchlist_url: &str,
    ) -> String {
        let aircraft_name = flight_data.aircraft.display_name();

        // Format takeoff time
        let takeoff_display = flight_data
            .takeoff_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Detected airborne".to_string());

        // Format landing time
        let landing_display = flight_data
            .landing_time
            .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| {
                if flight_data.timed_out {
                    "Signal lost".to_string()
                } else {
                    "In progress".to_string()
                }
            });

        // Format duration
        let duration_display = flight_data
            .duration_seconds
            .map(format_duration)
            .unwrap_or_else(|| "N/A".to_string());

        // Format distance
        let distance_display = flight_data
            .total_distance_meters
            .map(format_distance)
            .unwrap_or_else(|| "N/A".to_string());

        // Format displacement
        let displacement_display = flight_data
            .max_displacement_meters
            .map(format_distance)
            .unwrap_or_else(|| "N/A".to_string());

        // Build departure info
        let departure_display = match (
            &flight_data.departure_airport,
            &flight_data.departure_airport_name,
        ) {
            (Some(ident), Some(name)) => format!("{} ({})", name, ident),
            (Some(ident), None) => ident.clone(),
            _ => "Unknown".to_string(),
        };

        // Build arrival info
        let arrival_display = match (
            &flight_data.arrival_airport,
            &flight_data.arrival_airport_name,
        ) {
            (Some(ident), Some(name)) => format!("{} ({})", name, ident),
            (Some(ident), None) => ident.clone(),
            _ => "Unknown".to_string(),
        };

        let status = if flight_data.timed_out {
            "Signal Lost"
        } else if flight_data.detected_airborne {
            "Completed (Detected Airborne)"
        } else {
            "Completed"
        };

        format!(
            r#"Hello {to_name},

An aircraft on your watchlist has completed a flight!

AIRCRAFT: {aircraft_name}
Aircraft Page: {aircraft_url}

STATUS: {status}

FLIGHT DETAILS
==============
Takeoff:         {takeoff_display}{takeoff_runway}
Landing:         {landing_display}{landing_runway}
Duration:        {duration_display}
Total Distance:  {distance_display}

Departure:       {departure_display}
Arrival:         {arrival_display}
Max Displacement: {displacement_display}

Aircraft Address: {hex_address}

ATTACHED FILES
==============
- KML File: Open in Google Earth or other mapping applications
- IGC File: Standard flight log format for analysis software

LINKS
=====
View Flight: {flight_url}
Manage Watchlist: {watchlist_url}

---
You received this email because you're watching this aircraft on SOAR.
To change your email preferences, visit: {watchlist_url}

Best regards,
The SOAR Team"#,
            to_name = to_name,
            aircraft_name = aircraft_name,
            aircraft_url = aircraft_url,
            status = status,
            takeoff_display = takeoff_display,
            takeoff_runway = flight_data
                .takeoff_runway
                .as_ref()
                .map(|r| format!(" (Runway {})", r))
                .unwrap_or_default(),
            landing_display = landing_display,
            landing_runway = flight_data
                .landing_runway
                .as_ref()
                .map(|r| format!(" (Runway {})", r))
                .unwrap_or_default(),
            duration_display = duration_display,
            distance_display = distance_display,
            departure_display = departure_display,
            arrival_display = arrival_display,
            displacement_display = displacement_display,
            hex_address = flight_data.aircraft.hex_address,
            flight_url = flight_url,
            watchlist_url = watchlist_url,
        )
    }

    /// Send notification email when a new user signs up
    /// Sends to the same address as data load reports (EMAIL_TO env var)
    pub async fn send_user_signup_notification(
        &self,
        user_first_name: &str,
        user_last_name: &str,
        user_email: &str,
        club_id: Option<uuid::Uuid>,
    ) -> Result<Response> {
        // Get the admin notification email (same as data load reports)
        let to_email = std::env::var("EMAIL_TO")
            .map_err(|_| anyhow::anyhow!("EMAIL_TO not set for admin notifications"))?;

        let environment = get_environment_name();
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let subject = format!(
            "{}New User Signup - {} {}",
            get_staging_prefix(),
            user_first_name,
            user_last_name
        );

        let club_info = club_id
            .map(|id| format!("Club ID: {}", id))
            .unwrap_or_else(|| "Club: None".to_string());

        let html_body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 500px; margin: 0 auto; background-color: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); overflow: hidden; }}
        .header {{ background: linear-gradient(135deg, #28a745 0%, #218838 100%); color: white; padding: 20px; text-align: center; }}
        .header h1 {{ margin: 0; font-size: 20px; }}
        .content {{ padding: 24px; }}
        .info-row {{ display: flex; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid #eee; }}
        .info-row:last-child {{ border-bottom: none; }}
        .info-label {{ color: #666; }}
        .info-value {{ font-weight: 500; }}
        .footer {{ background-color: #f8f9fa; padding: 16px; text-align: center; font-size: 12px; color: #666; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🎉 New User Signup</h1>
        </div>
        <div class="content">
            <p>A new user has registered on SOAR!</p>
            <div class="info-row">
                <span class="info-label">Name</span>
                <span class="info-value">{first_name} {last_name}</span>
            </div>
            <div class="info-row">
                <span class="info-label">Email</span>
                <span class="info-value">{email}</span>
            </div>
            <div class="info-row">
                <span class="info-label">{club_info}</span>
                <span class="info-value"></span>
            </div>
            <div class="info-row">
                <span class="info-label">Time</span>
                <span class="info-value">{timestamp}</span>
            </div>
        </div>
        <div class="footer">
            <p>Environment: {environment} | <a href="{base_url}">{base_url}</a></p>
        </div>
    </div>
</body>
</html>"#,
            first_name = html_escape(user_first_name),
            last_name = html_escape(user_last_name),
            email = html_escape(user_email),
            club_info = html_escape(&club_info),
            timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            environment = html_escape(&environment),
            base_url = base_url,
        );

        let text_body = format!(
            r#"New User Signup on SOAR

Name: {first_name} {last_name}
Email: {email}
{club_info}
Time: {timestamp}

Environment: {environment}
{base_url}"#,
            first_name = user_first_name,
            last_name = user_last_name,
            email = user_email,
            club_info = club_info,
            timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            environment = environment,
            base_url = base_url,
        );

        let email = Message::builder()
            .from(create_mailbox(&self.from_name, &self.from_email)?)
            .to(to_email.parse()?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text_body))
                    .singlepart(SinglePart::html(html_body)),
            )?;

        let response = self.mailer.send(email).await?;
        Ok(response)
    }
}

/// Format duration in seconds to human-readable string
fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Format distance in meters to human-readable string with nm and km
fn format_distance(meters: f64) -> String {
    let nm = meters / 1852.0;
    let km = meters / 1000.0;

    if nm >= 1.0 {
        format!("{:.1} nm ({:.1} km)", nm, km)
    } else {
        format!("{:.0} m", meters)
    }
}

/// Simple HTML escaping
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Data about a geofence exit event for email display
#[derive(Debug, Clone)]
pub struct GeofenceExitEmailData {
    /// Geofence name
    pub geofence_name: String,
    /// Geofence ID (for link generation)
    pub geofence_id: uuid::Uuid,
    /// Flight ID (for link generation)
    pub flight_id: uuid::Uuid,
    /// Aircraft information
    pub aircraft: AircraftEmailData,
    /// Exit time
    pub exit_time: DateTime<Utc>,
    /// Exit location
    pub exit_latitude: f64,
    pub exit_longitude: f64,
    /// Exit altitude (MSL feet)
    pub exit_altitude_msl_ft: Option<i32>,
    /// Layer that was exited
    pub exit_layer_floor_ft: i32,
    pub exit_layer_ceiling_ft: i32,
    pub exit_layer_radius_nm: f64,
}

impl EmailService {
    /// Send geofence exit alert email
    pub async fn send_geofence_exit_email(
        &self,
        to_email: &str,
        to_name: &str,
        exit_data: &GeofenceExitEmailData,
    ) -> Result<Response> {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let geofence_url = format!("{}/geofences/{}", base_url, exit_data.geofence_id);
        let flight_url = format!("{}/flights/{}", base_url, exit_data.flight_id);
        let aircraft_url = format!("{}/aircraft/{}", base_url, exit_data.aircraft.id);

        let aircraft_name = exit_data.aircraft.display_name();
        let environment = get_environment_name();

        let subject = format!(
            "{}Geofence Alert: {} exited \"{}\"",
            get_staging_prefix(),
            aircraft_name,
            exit_data.geofence_name
        );

        // Build HTML email
        let html_body = self.build_geofence_exit_html(
            to_name,
            exit_data,
            &geofence_url,
            &flight_url,
            &aircraft_url,
            &environment,
        );

        // Build plain text fallback
        let text_body = self.build_geofence_exit_text(
            to_name,
            exit_data,
            &geofence_url,
            &flight_url,
            &aircraft_url,
        );

        let email = Message::builder()
            .from(create_mailbox(&self.from_name, &self.from_email)?)
            .to(create_mailbox(to_name, to_email)?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text_body))
                    .singlepart(SinglePart::html(html_body)),
            )?;

        let response = self.mailer.send(email).await?;
        Ok(response)
    }

    /// Build HTML body for geofence exit email
    fn build_geofence_exit_html(
        &self,
        to_name: &str,
        exit_data: &GeofenceExitEmailData,
        geofence_url: &str,
        flight_url: &str,
        aircraft_url: &str,
        environment: &str,
    ) -> String {
        let aircraft_name = exit_data.aircraft.display_name();
        let exit_time_display = exit_data
            .exit_time
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let altitude_display = exit_data
            .exit_altitude_msl_ft
            .map(|a| format!("{} ft MSL", a))
            .unwrap_or_else(|| "Unknown".to_string());

        let layer_display = format!(
            "{}-{} ft MSL, {} nm radius",
            exit_data.exit_layer_floor_ft,
            exit_data.exit_layer_ceiling_ft,
            exit_data.exit_layer_radius_nm
        );

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; background-color: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); overflow: hidden; }}
        .header {{ background: linear-gradient(135deg, #dc3545 0%, #c82333 100%); color: white; padding: 24px; text-align: center; }}
        .header h1 {{ margin: 0 0 8px 0; font-size: 24px; font-weight: 600; }}
        .header .geofence-name {{ font-size: 20px; font-weight: 500; opacity: 0.95; }}
        .alert-badge {{ display: inline-block; padding: 4px 12px; border-radius: 12px; font-size: 12px; font-weight: 600; color: white; background-color: #ffc107; margin-top: 8px; }}
        .content {{ padding: 24px; }}
        .greeting {{ font-size: 16px; margin-bottom: 16px; }}
        .alert-message {{ background-color: #fff3cd; border-left: 4px solid #ffc107; padding: 16px; border-radius: 4px; margin-bottom: 24px; }}
        .stats-grid {{ display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px; margin-bottom: 24px; }}
        .stat-card {{ background-color: #f8f9fa; border-radius: 8px; padding: 16px; border-left: 4px solid #dc3545; }}
        .stat-label {{ font-size: 12px; color: #666; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px; }}
        .stat-value {{ font-size: 18px; font-weight: 600; color: #1e3a5f; }}
        .stat-subvalue {{ font-size: 12px; color: #888; margin-top: 2px; }}
        .flight-details {{ margin-bottom: 24px; }}
        .detail-row {{ display: flex; justify-content: space-between; padding: 12px 0; border-bottom: 1px solid #eee; }}
        .detail-row:last-child {{ border-bottom: none; }}
        .detail-label {{ color: #666; }}
        .detail-value {{ font-weight: 500; text-align: right; }}
        .cta-button {{ display: inline-block; background: linear-gradient(135deg, #dc3545 0%, #c82333 100%); color: white; text-decoration: none; padding: 12px 24px; border-radius: 6px; font-weight: 600; margin-right: 8px; margin-bottom: 8px; }}
        .cta-button:hover {{ opacity: 0.9; }}
        .cta-secondary {{ background: white; color: #dc3545; border: 2px solid #dc3545; }}
        .footer {{ background-color: #f8f9fa; padding: 16px 24px; text-align: center; font-size: 12px; color: #666; }}
        .footer a {{ color: #dc3545; text-decoration: none; }}
        .footer a:hover {{ text-decoration: underline; }}
        @media (max-width: 480px) {{
            .stats-grid {{ grid-template-columns: 1fr; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Geofence Alert</h1>
            <div class="geofence-name">"{geofence_name}"</div>
            <div class="alert-badge">BOUNDARY EXITED</div>
        </div>
        <div class="content">
            <p class="greeting">Hello {to_name},</p>

            <div class="alert-message">
                <strong><a href="{aircraft_url}" style="color: #856404;">{aircraft_name}</a></strong> has exited the geofence boundary.
            </div>

            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-label">Exit Time</div>
                    <div class="stat-value">{exit_time}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">Altitude</div>
                    <div class="stat-value">{altitude}</div>
                </div>
            </div>

            <div class="flight-details">
                <div class="detail-row">
                    <span class="detail-label">Exit Location</span>
                    <span class="detail-value">{lat:.4}°, {lng:.4}°</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Layer Exited</span>
                    <span class="detail-value">{layer}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">Aircraft Address</span>
                    <span class="detail-value">{hex_address}</span>
                </div>
            </div>

            <div style="text-align: center; margin-bottom: 16px;">
                <a href="{flight_url}" class="cta-button">View Flight</a>
                <a href="{geofence_url}" class="cta-button cta-secondary">View Geofence</a>
            </div>
        </div>
        <div class="footer">
            <p>You received this email because you're subscribed to geofence alerts for "{geofence_name}".</p>
            <p><a href="{geofence_url}">Manage subscription</a> | Environment: {environment}</p>
            <p style="margin-top: 12px; color: #999;">Generated by SOAR Flight Tracker</p>
        </div>
    </div>
</body>
</html>"#,
            geofence_name = html_escape(&exit_data.geofence_name),
            to_name = html_escape(to_name),
            aircraft_url = aircraft_url,
            aircraft_name = html_escape(&aircraft_name),
            exit_time = html_escape(&exit_time_display),
            altitude = html_escape(&altitude_display),
            lat = exit_data.exit_latitude,
            lng = exit_data.exit_longitude,
            layer = html_escape(&layer_display),
            hex_address = html_escape(&exit_data.aircraft.hex_address),
            flight_url = flight_url,
            geofence_url = geofence_url,
            environment = html_escape(environment),
        )
    }

    /// Build plain text body for geofence exit email
    fn build_geofence_exit_text(
        &self,
        to_name: &str,
        exit_data: &GeofenceExitEmailData,
        geofence_url: &str,
        flight_url: &str,
        aircraft_url: &str,
    ) -> String {
        let aircraft_name = exit_data.aircraft.display_name();
        let exit_time_display = exit_data
            .exit_time
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let altitude_display = exit_data
            .exit_altitude_msl_ft
            .map(|a| format!("{} ft MSL", a))
            .unwrap_or_else(|| "Unknown".to_string());

        let layer_display = format!(
            "{}-{} ft MSL, {} nm radius",
            exit_data.exit_layer_floor_ft,
            exit_data.exit_layer_ceiling_ft,
            exit_data.exit_layer_radius_nm
        );

        format!(
            r#"Hello {to_name},

GEOFENCE ALERT: BOUNDARY EXITED
===============================

Aircraft "{aircraft_name}" has exited the geofence "{geofence_name}".

EXIT DETAILS
============
Exit Time:     {exit_time}
Altitude:      {altitude}
Location:      {lat:.4}°, {lng:.4}°
Layer Exited:  {layer}
Aircraft Hex:  {hex_address}

LINKS
=====
View Flight:   {flight_url}
View Aircraft: {aircraft_url}
View Geofence: {geofence_url}

---
You received this email because you're subscribed to geofence alerts for "{geofence_name}".
To manage your subscription, visit: {geofence_url}

Best regards,
The SOAR Team"#,
            to_name = to_name,
            aircraft_name = aircraft_name,
            geofence_name = exit_data.geofence_name,
            exit_time = exit_time_display,
            altitude = altitude_display,
            lat = exit_data.exit_latitude,
            lng = exit_data.exit_longitude,
            layer = layer_display,
            hex_address = exit_data.aircraft.hex_address,
            flight_url = flight_url,
            aircraft_url = aircraft_url,
            geofence_url = geofence_url,
        )
    }
}
