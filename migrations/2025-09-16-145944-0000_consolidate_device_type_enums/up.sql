-- Consolidate address_type and device_type_enum into a single device_type enum
-- This removes the duplicate address_type enum and renames device_type_enum to device_type

-- Step 1: Update the fixes table to use device_type_enum instead of address_type
-- Map the enum values correctly from address_type to device_type_enum
ALTER TABLE fixes ALTER COLUMN device_type TYPE device_type_enum USING
    CASE device_type::text
        WHEN 'unknown_type' THEN 'unknown'::device_type_enum
        WHEN 'icao_address' THEN 'icao'::device_type_enum
        WHEN 'flarm_id' THEN 'flarm'::device_type_enum
        WHEN 'ogn_tracker' THEN 'ogn'::device_type_enum
        -- Handle any legacy CamelCase values that might exist
        WHEN 'Unknown' THEN 'unknown'::device_type_enum
        WHEN 'Icao' THEN 'icao'::device_type_enum
        WHEN 'Flarm' THEN 'flarm'::device_type_enum
        WHEN 'OgnTracker' THEN 'ogn'::device_type_enum
        ELSE 'unknown'::device_type_enum
    END;

-- Step 2: Drop the now-unused address_type enum
DROP TYPE address_type;

-- Step 3: Rename device_type_enum to device_type
ALTER TYPE device_type_enum RENAME TO device_type;
