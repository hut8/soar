-- Consolidate address_type and device_type_enum into a single device_type enum
-- This removes the duplicate address_type enum and renames device_type_enum to device_type

-- Step 1: Update the fixes table to use device_type_enum instead of address_type
-- Map the enum values correctly
ALTER TABLE fixes ALTER COLUMN device_type TYPE device_type_enum USING
    CASE device_type::text
        WHEN 'unknown_type' THEN 'unknown_type'::device_type_enum
        WHEN 'icao_address' THEN 'icao_address'::device_type_enum
        WHEN 'flarm_id' THEN 'flarm_id'::device_type_enum
        WHEN 'ogn_tracker' THEN 'ogn_tracker'::device_type_enum
        ELSE 'unknown_type'::device_type_enum
    END;

-- Step 2: Drop the now-unused address_type enum
DROP TYPE address_type;

-- Step 3: Rename device_type_enum to device_type
ALTER TYPE device_type_enum RENAME TO device_type;