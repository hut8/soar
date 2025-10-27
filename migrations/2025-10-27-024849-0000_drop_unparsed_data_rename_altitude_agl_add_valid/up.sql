-- Drop unparsed_data column from fixes table (it's already in aprs_messages)
ALTER TABLE fixes DROP COLUMN IF EXISTS unparsed_data;

-- Rename altitude_agl to altitude_agl_feet for consistency
ALTER TABLE fixes RENAME COLUMN altitude_agl TO altitude_agl_feet;

-- Add altitude_agl_valid boolean column
-- Set to true if altitude_agl_feet is not null (meaning it was already calculated)
-- Default to false for new rows
ALTER TABLE fixes
ADD COLUMN altitude_agl_valid BOOLEAN NOT NULL DEFAULT false;

-- Set altitude_agl_valid to true for all existing rows where altitude_agl_feet is not null
UPDATE fixes
SET altitude_agl_valid = true
WHERE altitude_agl_feet IS NOT NULL;

-- Update the index name to match the new column name
DROP INDEX IF EXISTS idx_fixes_altitude_agl;
CREATE INDEX IF NOT EXISTS idx_fixes_altitude_agl_feet ON fixes(altitude_agl_feet);

-- Add index for querying by altitude_agl_valid
CREATE INDEX IF NOT EXISTS idx_fixes_altitude_agl_valid ON fixes(altitude_agl_valid) WHERE altitude_agl_valid = false;
