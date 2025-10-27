-- Remove the altitude_agl_valid index
DROP INDEX CONCURRENTLY IF EXISTS idx_fixes_altitude_agl_valid;
