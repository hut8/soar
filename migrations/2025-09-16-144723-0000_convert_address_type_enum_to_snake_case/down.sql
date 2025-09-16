-- Revert address_type enum from snake_case back to CamelCase variants
-- WARNING: This will convert data back to the original CamelCase format

-- Create a new enum with CamelCase values
CREATE TYPE address_type_camel AS ENUM ('OgnTracker', 'IcaoAddress', 'FlarmId', 'UnknownType');

-- Update the table to use the CamelCase enum type
ALTER TABLE fixes ALTER COLUMN device_type TYPE address_type_camel USING
    CASE device_type::text
        WHEN 'ogn_tracker' THEN 'OgnTracker'::address_type_camel
        WHEN 'icao_address' THEN 'IcaoAddress'::address_type_camel
        WHEN 'flarm_id' THEN 'FlarmId'::address_type_camel
        WHEN 'unknown_type' THEN 'UnknownType'::address_type_camel
        ELSE 'UnknownType'::address_type_camel
    END;

-- Drop the snake_case enum and rename the CamelCase one
DROP TYPE address_type;
ALTER TYPE address_type_camel RENAME TO address_type;