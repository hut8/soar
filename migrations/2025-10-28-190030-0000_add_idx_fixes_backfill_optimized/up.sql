-- Add optimized partial index for AGL backfill queries
-- This index supports the query pattern used by get_fixes_needing_backfill():
-- WHERE altitude_agl_valid = false AND altitude_msl_feet IS NOT NULL AND is_active = true
-- ORDER BY timestamp ASC
--
-- NOTE: This is NOT CONCURRENTLY because Diesel migrations run in transactions
CREATE INDEX idx_fixes_backfill_optimized
ON fixes (timestamp ASC)
WHERE altitude_agl_valid = false
  AND altitude_msl_feet IS NOT NULL
  AND is_active = true;
