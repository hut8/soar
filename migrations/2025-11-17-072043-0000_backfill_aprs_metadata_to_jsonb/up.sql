-- Backfill APRS-specific metadata from individual columns into source_metadata JSONB
-- This migration moves protocol-specific data to allow future ADS-B fields without hitting 32-column limit

-- Update all existing fixes with APRS metadata
-- Only backfill non-NULL values to keep JSONB compact
UPDATE fixes
SET source_metadata = jsonb_build_object(
    'protocol', 'aprs',
    'snr_db', snr_db,
    'bit_errors_corrected', bit_errors_corrected,
    'freq_offset_khz', freq_offset_khz,
    'gnss_horizontal_resolution', gnss_horizontal_resolution,
    'gnss_vertical_resolution', gnss_vertical_resolution
) - ARRAY(
    SELECT key
    FROM jsonb_each(jsonb_build_object(
        'protocol', 'aprs',
        'snr_db', snr_db,
        'bit_errors_corrected', bit_errors_corrected,
        'freq_offset_khz', freq_offset_khz,
        'gnss_horizontal_resolution', gnss_horizontal_resolution,
        'gnss_vertical_resolution', gnss_vertical_resolution
    ))
    WHERE value = 'null'::jsonb
)
WHERE source_metadata IS NULL
  AND (
    snr_db IS NOT NULL
    OR bit_errors_corrected IS NOT NULL
    OR freq_offset_khz IS NOT NULL
    OR gnss_horizontal_resolution IS NOT NULL
    OR gnss_vertical_resolution IS NOT NULL
  );

COMMENT ON COLUMN fixes.source_metadata IS 'Protocol-specific metadata stored as JSONB. For APRS: snr_db, bit_errors_corrected, freq_offset_khz, gnss_*_resolution. For ADS-B: nic, nac_p, nac_v, sil, emergency_status, on_ground, etc.';
