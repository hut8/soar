-- Convert device_address from hex string to 32-bit signed integer
-- First, add a temporary column to store the converted values
ALTER TABLE fixes ADD COLUMN device_address_temp INTEGER;

-- Convert hex strings to integers
-- Use CASE to handle potential conversion errors gracefully
UPDATE fixes
SET device_address_temp = CASE
    WHEN device_address ~ '^[0-9A-Fa-f]+$' THEN
        ('x' || lpad(device_address, 8, '0'))::bit(32)::int
    ELSE
        0  -- Default value for invalid hex strings
END;

-- Drop the old column
ALTER TABLE fixes DROP COLUMN device_address;

-- Rename the temp column to the original name
ALTER TABLE fixes RENAME COLUMN device_address_temp TO device_address;

-- Add NOT NULL constraint
ALTER TABLE fixes ALTER COLUMN device_address SET NOT NULL;