-- Recreate the unique index on devices(address)
-- Note: This will fail if there are devices with the same address but different address_type
-- This migration should not be run in production as it creates a conflicting constraint

CREATE UNIQUE INDEX idx_devices_address_unique ON devices(address);
