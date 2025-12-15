use anyhow::Result;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Message, header::ContentType},
    transport::smtp::{authentication::Credentials, response::Response},
};

/// Get the staging prefix for email subjects
/// Returns "[STAGING] " if SOAR_ENV=staging, empty string otherwise
fn get_staging_prefix() -> &'static str {
    match std::env::var("SOAR_ENV").unwrap_or_default().as_str() {
        "staging" => "[STAGING] ",
        _ => "",
    }
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
            // Port 465 uses implicit TLS (TLS wrapper)
            // Connection starts with TLS immediately, no STARTTLS upgrade
            tracing::info!("Using implicit TLS (TLS wrapper) for port 465");
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_server)
                .port(smtp_port)
                .credentials(creds)
                .tls(lettre::transport::smtp::client::Tls::Wrapper(
                    lettre::transport::smtp::client::TlsParameters::new(smtp_server.clone())?,
                ))
                .build()
        } else {
            // Port 587 and others use STARTTLS
            // Connection starts plain and upgrades to TLS
            tracing::info!("Using STARTTLS for port {}", smtp_port);
            AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_server)?
                .port(smtp_port)
                .credentials(creds)
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
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
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
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", to_name, to_email).parse()?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        let response = self.mailer.send(email).await?;
        Ok(response)
    }
}
