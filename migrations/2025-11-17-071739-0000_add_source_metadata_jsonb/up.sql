-- Add source_metadata JSONB column to fixes table for protocol-specific metadata
-- This allows us to consolidate APRS-specific columns and add ADS-B metadata
-- without hitting the 32-column PostgreSQL limit

ALTER TABLE fixes ADD COLUMN source_metadata JSONB;

-- Create GIN index for fast JSONB queries
CREATE INDEX idx_fixes_source_metadata ON fixes USING GIN (source_metadata);

-- Create partial index for protocol type (fast filtering by protocol)
CREATE INDEX idx_fixes_protocol ON fixes ((source_metadata->>'protocol'))
  WHERE source_metadata IS NOT NULL;

COMMENT ON COLUMN fixes.source_metadata IS 'Protocol-specific metadata stored as JSONB. For APRS: snr_db, bit_errors_corrected, freq_offset_khz, gnss_*_resolution. For ADS-B: nic, nac_p, nac_v, sil, emergency_status, on_ground, etc.';
