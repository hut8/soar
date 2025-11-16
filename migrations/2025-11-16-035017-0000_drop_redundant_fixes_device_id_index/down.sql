-- Recreate the single-column device_id index on fixes table
-- Note: This is a rollback migration. The index is redundant with idx_fixes_device_received_at,
-- but we recreate it here for migration reversibility

CREATE INDEX fixes_device_id_idx ON fixes (device_id);
