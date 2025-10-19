-- Convert address_type enum from CamelCase to snake_case variants
-- This fixes the enum to match the expected snake_case format for diesel_derive_enum

-- Create a new enum with snake_case values
CREATE TYPE address_type_new AS ENUM ('ogn_tracker', 'icao_address', 'flarm_id', 'unknown_type');

-- Update the table to use the new enum type, mapping CamelCase to snake_case
ALTER TABLE fixes ALTER COLUMN device_type TYPE address_type_new USING
    CASE device_type::text
        WHEN 'OgnTracker' THEN 'ogn_tracker'::address_type_new
        WHEN 'IcaoAddress' THEN 'icao_address'::address_type_new
        WHEN 'FlarmId' THEN 'flarm_id'::address_type_new
        WHEN 'UnknownType' THEN 'unknown_type'::address_type_new
        ELSE 'unknown_type'::address_type_new
    END;

-- Drop the old enum and rename the new one
DROP TYPE address_type;
ALTER TYPE address_type_new RENAME TO address_type;
