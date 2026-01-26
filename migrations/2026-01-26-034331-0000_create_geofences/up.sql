-- Create geofences table
-- Stores "upside-down birthday cake" shaped geofences with multiple altitude layers
-- Each layer has its own radius, similar to Class B airspace

CREATE TABLE geofences (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    -- Center point of all layers (PostGIS geography for accurate distance calculations)
    center GEOGRAPHY(POINT, 4326) NOT NULL,
    -- Maximum radius across all layers (meters) for efficient bounding box queries
    max_radius_meters DOUBLE PRECISION NOT NULL,
    -- Layers stored as JSONB array
    -- Each layer: { "floor_ft": 0, "ceiling_ft": 5000, "radius_nm": 5.0 }
    -- Altitudes are MSL (Mean Sea Level)
    layers JSONB NOT NULL DEFAULT '[]',
    -- User who created/owns the geofence
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Optional club association
    club_id UUID REFERENCES clubs(id) ON DELETE SET NULL,
    -- Soft delete support
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Spatial index for center point queries (find geofences near a location)
CREATE INDEX idx_geofences_center ON geofences USING GIST (center);

-- Index for user queries (list user's geofences)
CREATE INDEX idx_geofences_owner ON geofences (owner_user_id) WHERE deleted_at IS NULL;

-- Index for club queries (list club's geofences)
CREATE INDEX idx_geofences_club ON geofences (club_id) WHERE deleted_at IS NULL AND club_id IS NOT NULL;

-- Trigger for updated_at
CREATE TRIGGER set_geofences_updated_at
    BEFORE UPDATE ON geofences
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
