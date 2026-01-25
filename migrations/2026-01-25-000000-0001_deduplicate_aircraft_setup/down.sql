-- Rollback: Drop merge mapping table
-- Note: This does NOT undo the FK updates - those would need manual restoration
DROP TABLE IF EXISTS aircraft_merge_mapping;
