-- Revert address_type enum from snake_case back to CamelCase variants
-- WARNING: This will convert data back to the original CamelCase format

-- Create a new enum with CamelCase values
CREATE TYPE address_type_old AS ENUM ('OgnTracker', 'IcaoAddress', 'FlarmId', 'UnknownType');

-- Update the table to use the CamelCase enum type
ALTER TABLE fixes ALTER COLUMN device_type TYPE address_type_old USING
    CASE device_type::text
        WHEN 'ogn_tracker' THEN 'OgnTracker'::address_type_old
        WHEN 'icao_address' THEN 'IcaoAddress'::address_type_old
        WHEN 'flarm_id' THEN 'FlarmId'::address_type_old
        WHEN 'unknown_type' THEN 'UnknownType'::address_type_old
        ELSE 'UnknownType'::address_type_old
    END;

-- Drop the snake_case enum and rename the CamelCase one
DROP TYPE address_type;
ALTER TYPE address_type_old RENAME TO address_type;