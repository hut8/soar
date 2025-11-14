use anyhow::Result;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Message, header::ContentType},
    transport::smtp::{authentication::Credentials, response::Response},
};

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

        // For test/development environments (like Mailpit), allow insecure SMTP
        // Check if we're using a local/test SMTP server (port 1025 is Mailpit's default)
        let mailer = if smtp_port == 1025 {
            // Use builder for insecure local SMTP (Mailpit)
            tracing::info!("Using insecure SMTP connection for port 1025 (Mailpit)");
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_server)
                .port(smtp_port)
                .credentials(creds)
                .build()
        } else {
            // Use relay (with TLS) for production SMTP servers
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

        let subject = "Password Reset Request - SOAR";
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

        let subject = "Verify Your Email Address - SOAR";
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
