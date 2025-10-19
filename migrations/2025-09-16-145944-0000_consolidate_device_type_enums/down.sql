-- Revert the enum consolidation
-- This recreates address_type enum and reverts device_type back to device_type_enum

-- Step 1: Rename device_type back to device_type_enum
ALTER TYPE device_type RENAME TO device_type_enum;

-- Step 2: Recreate the address_type enum with snake_case values
CREATE TYPE address_type AS ENUM ('unknown_type', 'icao_address', 'flarm_id', 'ogn_tracker');

-- Step 3: Update the fixes table to use address_type again
ALTER TABLE fixes ALTER COLUMN device_type TYPE address_type USING
    CASE device_type::text
        WHEN 'unknown' THEN 'unknown_type'::address_type
        WHEN 'icao' THEN 'icao_address'::address_type
        WHEN 'flarm' THEN 'flarm_id'::address_type
        WHEN 'ogn' THEN 'ogn_tracker'::address_type
        ELSE 'unknown_type'::address_type
    END;
