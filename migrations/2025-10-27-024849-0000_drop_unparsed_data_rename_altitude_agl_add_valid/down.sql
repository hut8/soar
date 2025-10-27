-- Remove the altitude_agl_valid indexes
DROP INDEX IF EXISTS idx_fixes_altitude_agl_valid;
DROP INDEX IF EXISTS idx_fixes_altitude_agl_feet;

-- Re-create the old index name
CREATE INDEX IF NOT EXISTS idx_fixes_altitude_agl ON fixes(altitude_agl_feet);

-- Drop altitude_agl_valid column
ALTER TABLE fixes DROP COLUMN IF EXISTS altitude_agl_valid;

-- Rename altitude_agl_feet back to altitude_agl
ALTER TABLE fixes RENAME COLUMN altitude_agl_feet TO altitude_agl;

-- Re-add unparsed_data column (will be empty)
ALTER TABLE fixes ADD COLUMN IF NOT EXISTS unparsed_data VARCHAR;
