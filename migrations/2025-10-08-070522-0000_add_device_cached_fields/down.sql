-- Revert cached device fields

DROP INDEX IF EXISTS idx_devices_last_fix_at;

ALTER TABLE devices DROP COLUMN IF EXISTS last_fix_at;
ALTER TABLE devices DROP COLUMN IF EXISTS aircraft_type_ogn;
