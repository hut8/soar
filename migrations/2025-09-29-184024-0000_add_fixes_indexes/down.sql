-- Remove indexes added for fixes table performance

DROP INDEX IF EXISTS fixes_device_received_at_idx;
DROP INDEX IF EXISTS fixes_received_at_idx;
