-- Remove from_adsbx_ddb column
ALTER TABLE aircraft DROP COLUMN IF EXISTS from_adsbx_ddb;

-- Rename from_ogn_ddb back to from_ddb
ALTER TABLE aircraft RENAME COLUMN from_ogn_ddb TO from_ddb;
