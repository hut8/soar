CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_stripe_webhook_events_processed ON stripe_webhook_events (processed) WHERE NOT processed;
