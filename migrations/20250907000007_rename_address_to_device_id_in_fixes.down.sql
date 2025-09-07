-- Add down migration script here
-- =========================================================
-- Revert renaming of device columns back to address in fixes table
-- =========================================================

-- Rename the columns back to the original names
ALTER TABLE fixes RENAME COLUMN device_id TO address;
ALTER TABLE fixes RENAME COLUMN device_type TO address_type;