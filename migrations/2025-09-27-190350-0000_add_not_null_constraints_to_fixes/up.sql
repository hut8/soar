-- Make via, device_address, address_type, device_id NOT NULL on fixes table
-- First, delete any rows with NULL values in these columns (just in case)

DELETE FROM fixes WHERE via IS NULL
   OR device_address IS NULL
   OR address_type IS NULL
   OR device_id IS NULL;

-- Add NOT NULL constraints
ALTER TABLE fixes ALTER COLUMN via SET NOT NULL;
ALTER TABLE fixes ALTER COLUMN device_address SET NOT NULL;
ALTER TABLE fixes ALTER COLUMN address_type SET NOT NULL;
ALTER TABLE fixes ALTER COLUMN device_id SET NOT NULL;
