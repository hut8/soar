-- Drop the redundant single-column device_id index on fixes table
-- The composite index idx_fixes_device_received_at (device_id, received_at DESC)
-- can serve all queries that fixes_device_id_idx can, making it redundant

DROP INDEX IF EXISTS fixes_device_id_idx;
