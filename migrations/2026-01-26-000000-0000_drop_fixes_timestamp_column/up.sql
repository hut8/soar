-- Drop the redundant timestamp column from fixes table
-- The timestamp column was always set to the same value as received_at
-- (see src/fixes.rs:125 "For now, use received_at as the packet timestamp")
-- The table is partitioned by received_at, so that is the canonical timestamp

ALTER TABLE fixes DROP COLUMN timestamp;
