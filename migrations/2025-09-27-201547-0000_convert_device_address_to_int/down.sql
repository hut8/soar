-- Revert device_address from 32-bit signed integer back to hex string
-- First, add a temporary column to store the converted values
ALTER TABLE fixes ADD COLUMN device_address_temp VARCHAR;

-- Convert integers back to hex strings (uppercase, 6 characters)
UPDATE fixes
SET device_address_temp = CASE
    WHEN device_address >= 0 THEN
        upper(lpad(to_hex(device_address), 6, '0'))
    ELSE
        '000000'  -- Default value for negative integers
END;

-- Drop the old column
ALTER TABLE fixes DROP COLUMN device_address;

-- Rename the temp column to the original name
ALTER TABLE fixes RENAME COLUMN device_address_temp TO device_address;

-- Add NOT NULL constraint
ALTER TABLE fixes ALTER COLUMN device_address SET NOT NULL;