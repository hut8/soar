-- Re-add region_code column to locations table
ALTER TABLE locations ADD COLUMN region_code TEXT;

-- Note: Original data cannot be recovered. This column would need to be repopulated
-- from the FAA aircraft registry if needed.
