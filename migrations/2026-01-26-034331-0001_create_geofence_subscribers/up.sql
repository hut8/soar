-- Create geofence_subscribers table
-- Links users to geofences they want to receive alerts for
-- Follows the watchlist composite-key pattern

CREATE TABLE geofence_subscribers (
    geofence_id UUID NOT NULL REFERENCES geofences(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Email preference for this subscription
    send_email BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (geofence_id, user_id)
);

-- Index for user queries (list user's subscriptions)
CREATE INDEX idx_geofence_subscribers_user ON geofence_subscribers (user_id);

-- Partial index for email notifications (only rows with send_email=true)
CREATE INDEX idx_geofence_subscribers_email ON geofence_subscribers (geofence_id) WHERE send_email = TRUE;

-- Trigger for updated_at
CREATE TRIGGER set_geofence_subscribers_updated_at
    BEFORE UPDATE ON geofence_subscribers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
