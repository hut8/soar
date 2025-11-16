-- Remove the conflicting unique index on devices(address)
-- This index conflicts with the existing unique constraint on (address_type, address)
-- which allows the same address to exist with different address types.
--
-- The idx_devices_address_unique index was incorrectly added in migration
-- 2025-10-16-232436-0000_fix_device_address_unique and prevents valid use cases
-- where the same address appears with different types (e.g., ICAO vs FLARM).
--
-- The correct unique constraint is devices_address_type_address_unique on
-- (address_type, address) which allows same address with different types.

DROP INDEX IF EXISTS idx_devices_address_unique;
