-- Rollback: Restore APRS-specific columns
-- Note: This recreates the columns but does NOT restore data
-- Data recovery would require extracting from source_metadata JSONB

ALTER TABLE fixes ADD COLUMN snr_db REAL;
ALTER TABLE fixes ADD COLUMN bit_errors_corrected INTEGER;
ALTER TABLE fixes ADD COLUMN freq_offset_khz REAL;
ALTER TABLE fixes ADD COLUMN gnss_horizontal_resolution REAL;
ALTER TABLE fixes ADD COLUMN gnss_vertical_resolution REAL;

-- Restore data from source_metadata JSONB if available
UPDATE fixes
SET snr_db = (source_metadata->>'snr_db')::REAL,
    bit_errors_corrected = (source_metadata->>'bit_errors_corrected')::INTEGER,
    freq_offset_khz = (source_metadata->>'freq_offset_khz')::REAL,
    gnss_horizontal_resolution = (source_metadata->>'gnss_horizontal_resolution')::REAL,
    gnss_vertical_resolution = (source_metadata->>'gnss_vertical_resolution')::REAL
WHERE source_metadata IS NOT NULL
  AND source_metadata IS NOT NULL;
