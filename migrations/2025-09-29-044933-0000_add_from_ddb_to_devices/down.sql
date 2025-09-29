-- Rollback: remove from_ddb column and its index from devices table
DROP INDEX IF EXISTS idx_devices_from_ddb;
ALTER TABLE devices DROP COLUMN from_ddb;
