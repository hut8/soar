-- Create club_tow_fees table for managing tow pricing based on altitude
CREATE TABLE club_tow_fees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    club_id UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    max_altitude INTEGER, -- NULL represents "anything above the highest specified altitude"
    cost NUMERIC(10, 2) NOT NULL CHECK (cost >= 0), -- Cost in currency (e.g., USD)
    modified_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Ensure each club has unique altitude tiers (including non-NULL values)
CREATE UNIQUE INDEX idx_club_tow_fees_club_altitude
    ON club_tow_fees (club_id, max_altitude)
    WHERE max_altitude IS NOT NULL;

-- Ensure only one NULL max_altitude per club (the fallback tier)
CREATE UNIQUE INDEX idx_club_tow_fees_club_null_altitude
    ON club_tow_fees (club_id)
    WHERE max_altitude IS NULL;

-- Index for lookups by club
CREATE INDEX idx_club_tow_fees_club_id ON club_tow_fees (club_id);
