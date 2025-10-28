-- Remove the optimized partial index for AGL backfill queries
DROP INDEX IF EXISTS idx_fixes_backfill_optimized;
