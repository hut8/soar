-- Convert address_type enum from CamelCase to snake_case variants
-- This fixes the enum to match the expected snake_case format for diesel_derive_enum

-- First, add new snake_case enum values
ALTER TYPE address_type ADD VALUE IF NOT EXISTS 'ogn_tracker';
ALTER TYPE address_type ADD VALUE IF NOT EXISTS 'icao_address';
ALTER TYPE address_type ADD VALUE IF NOT EXISTS 'flarm_id';
ALTER TYPE address_type ADD VALUE IF NOT EXISTS 'unknown_type';

-- Update existing data from CamelCase to snake_case
UPDATE fixes SET device_type = 'ogn_tracker'::address_type WHERE device_type = 'OgnTracker'::address_type;
UPDATE fixes SET device_type = 'icao_address'::address_type WHERE device_type = 'IcaoAddress'::address_type;
UPDATE fixes SET device_type = 'flarm_id'::address_type WHERE device_type = 'FlarmId'::address_type;
UPDATE fixes SET device_type = 'unknown_type'::address_type WHERE device_type = 'UnknownType'::address_type;

-- Remove old CamelCase enum values (this will fail if any data still references them)
-- Note: PostgreSQL doesn't support removing enum values directly, so we'll recreate the enum

-- Create a new enum with only snake_case values
CREATE TYPE address_type_new AS ENUM ('ogn_tracker', 'icao_address', 'flarm_id', 'unknown_type');

-- Update the table to use the new enum type
ALTER TABLE fixes ALTER COLUMN device_type TYPE address_type_new USING device_type::text::address_type_new;

-- Drop the old enum and rename the new one
DROP TYPE address_type;
ALTER TYPE address_type_new RENAME TO address_type;