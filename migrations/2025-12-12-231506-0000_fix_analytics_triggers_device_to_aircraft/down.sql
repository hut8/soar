-- This migration fixes broken triggers and cannot be safely reverted.
-- Rolling back would restore broken triggers that reference non-existent device_id columns.
-- If you need to revert, you should restore the broken state manually, but this is not recommended.

-- NO-OP: This migration cannot be safely reverted
