-- Add indexes to improve query performance on fixes table

-- Index for queries filtering by received_at (used in bounding box queries)
CREATE INDEX IF NOT EXISTS fixes_received_at_idx ON fixes (received_at);

-- Composite index for queries filtering by device_id and ordering by received_at
-- This optimizes queries that fetch recent fixes for specific devices
CREATE INDEX IF NOT EXISTS fixes_device_received_at_idx ON fixes (device_id, received_at DESC);
