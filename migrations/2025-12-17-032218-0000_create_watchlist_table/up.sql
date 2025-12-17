-- Create watchlist table
-- Tracks which aircraft each user is watching and their email preferences
CREATE TABLE watchlist (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    aircraft_id UUID NOT NULL REFERENCES aircraft(id) ON DELETE CASCADE,
    send_email BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, aircraft_id)
);

-- Index for querying user's watchlist
CREATE INDEX idx_watchlist_user_id ON watchlist(user_id);

-- Index for querying which users watch an aircraft (for email notifications)
CREATE INDEX idx_watchlist_aircraft_id ON watchlist(aircraft_id);

-- Partial index for email notifications (only rows with send_email=true)
CREATE INDEX idx_watchlist_send_email ON watchlist(aircraft_id) WHERE send_email = TRUE;

-- Create updated_at trigger
CREATE TRIGGER set_watchlist_updated_at
    BEFORE UPDATE ON watchlist
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
