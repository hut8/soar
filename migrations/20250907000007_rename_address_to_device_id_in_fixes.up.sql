-- Add up migration script here
-- =========================================================
-- Rename address columns to device_id/device_type in fixes table
-- =========================================================

-- Rename the columns to match the new terminology
ALTER TABLE fixes RENAME COLUMN address TO device_id;
ALTER TABLE fixes RENAME COLUMN address_type TO device_type;

-- Update any indexes if needed (PostgreSQL should handle this automatically)