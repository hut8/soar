-- Revert altitude_msl_feet back to altitude_feet
ALTER TABLE fixes RENAME COLUMN altitude_msl_feet TO altitude_feet;
