-- Airspace Sync Log Table
-- Tracks airspace synchronization operations for auditing and incremental updates

CREATE TABLE airspace_sync_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Timing
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    -- Results
    success BOOLEAN,
    airspaces_fetched INTEGER DEFAULT 0,
    airspaces_inserted INTEGER DEFAULT 0,
    airspaces_updated INTEGER DEFAULT 0,
    error_message TEXT,

    -- Sync configuration
    countries_filter TEXT[],  -- NULL means global sync, otherwise specific country codes

    -- For incremental sync - timestamp used in updatedAfter parameter
    updated_after TIMESTAMPTZ
);

-- Index for finding last successful sync (used for incremental updates)
CREATE INDEX idx_airspace_sync_log_completed ON airspace_sync_log (completed_at DESC);

-- Index for filtering successful syncs
CREATE INDEX idx_airspace_sync_log_success ON airspace_sync_log (success) WHERE success = true;

-- Table comment
COMMENT ON TABLE airspace_sync_log IS 'Audit log for airspace synchronization operations from OpenAIP';
COMMENT ON COLUMN airspace_sync_log.updated_after IS 'The updatedAfter parameter used for incremental sync - filters OpenAIP results';
COMMENT ON COLUMN airspace_sync_log.countries_filter IS 'NULL for global sync, otherwise array of ISO 3166-1 alpha-2 country codes';
