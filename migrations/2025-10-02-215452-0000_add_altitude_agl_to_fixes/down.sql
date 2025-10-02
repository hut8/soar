-- Remove altitude_agl column from fixes table
DROP INDEX IF EXISTS idx_fixes_altitude_agl;

ALTER TABLE fixes
DROP COLUMN IF EXISTS altitude_agl;
