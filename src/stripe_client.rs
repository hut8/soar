use anyhow::{Context, Result};
use stripe::Client;

/// Configuration for Stripe integration
#[derive(Clone)]
pub struct StripeConfig {
    pub client: Client,
    pub webhook_secret: String,
    /// Platform fee in basis points (e.g., 250 = 2.5%)
    pub platform_fee_bps: u32,
}

impl StripeConfig {
    /// Initialize Stripe configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let secret_key =
            std::env::var("STRIPE_SECRET_KEY").context("STRIPE_SECRET_KEY must be set")?;
        let webhook_secret =
            std::env::var("STRIPE_WEBHOOK_SECRET").context("STRIPE_WEBHOOK_SECRET must be set")?;
        let platform_fee_bps: u32 = std::env::var("STRIPE_PLATFORM_FEE_BPS")
            .unwrap_or_else(|_| "250".to_string())
            .parse()
            .context("STRIPE_PLATFORM_FEE_BPS must be a valid number")?;

        let client = Client::new(secret_key);

        Ok(Self {
            client,
            webhook_secret,
            platform_fee_bps,
        })
    }

    /// Calculate platform fee in cents for a given amount in cents
    pub fn calculate_platform_fee(&self, amount_cents: i32) -> i32 {
        ((amount_cents as i64 * self.platform_fee_bps as i64) / 10_000) as i32
    }
}

impl std::fmt::Debug for StripeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeConfig")
            .field("webhook_secret", &"[REDACTED]")
            .field("platform_fee_bps", &self.platform_fee_bps)
            .finish()
    }
}
