-- Add altitude_agl (Above Ground Level) column to fixes table
ALTER TABLE fixes
ADD COLUMN IF NOT EXISTS altitude_agl INTEGER;

-- Add index for queries filtering by AGL
CREATE INDEX IF NOT EXISTS idx_fixes_altitude_agl ON fixes(altitude_agl);
