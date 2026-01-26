-- Drop the redundant timestamp column from fixes table
-- The timestamp column was always set to the same value as received_at
-- The table is partitioned by received_at, so that is the canonical timestamp

ALTER TABLE fixes DROP COLUMN timestamp;
