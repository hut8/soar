-- Remove created_at, updated_at, and raw_data columns from receiver_statuses table
ALTER TABLE receiver_statuses DROP COLUMN IF EXISTS created_at;
ALTER TABLE receiver_statuses DROP COLUMN IF EXISTS updated_at;
ALTER TABLE receiver_statuses DROP COLUMN IF EXISTS raw_data;
