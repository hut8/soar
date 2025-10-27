-- Add index for querying by altitude_agl_valid
-- This index helps efficiently find fixes that need AGL altitude calculation
CREATE INDEX IF NOT EXISTS idx_fixes_altitude_agl_valid ON fixes(altitude_agl_valid) WHERE altitude_agl_valid = false;
