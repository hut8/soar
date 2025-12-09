-- Backfill APRS-specific metadata from individual columns into source_metadata JSONB
-- This migration moves protocol-specific data to allow future ADS-B fields without hitting 32-column limit

-- Update all existing fixes with APRS metadata
-- Only backfill non-NULL values to keep JSONB compact
-- Note: protocol field removed as it's redundant (determined from raw_messages.source)
UPDATE fixes
SET source_metadata = jsonb_build_object(
    'snr_db', snr_db,
    'bit_errors_corrected', bit_errors_corrected,
    'freq_offset_khz', freq_offset_khz,
    'gnss_horizontal_resolution', gnss_horizontal_resolution,
    'gnss_vertical_resolution', gnss_vertical_resolution
) - ARRAY(
    SELECT key
    FROM jsonb_each(jsonb_build_object(
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

COMMENT ON COLUMN fixes.source_metadata IS 'Protocol-specific metadata stored as JSONB. For OGN/APRS (protocol=aprs): snr_db, bit_errors_corrected, freq_offset_khz, gnss_*_resolution. For ADS-B (protocol=adsb): nic, nac_p, nac_v, sil, emergency_status, on_ground, etc.';
