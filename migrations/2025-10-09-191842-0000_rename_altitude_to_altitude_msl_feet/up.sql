-- Rename altitude_feet column to altitude_msl_feet for consistency
ALTER TABLE fixes RENAME COLUMN altitude_feet TO altitude_msl_feet;
