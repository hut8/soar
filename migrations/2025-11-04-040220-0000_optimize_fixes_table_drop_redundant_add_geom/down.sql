-- Reverse the fixes table optimization
-- This restores the dropped columns and removes the geometry column

-- Note: The geometry index is dropped in a separate migration (2025-11-04-042950-0000_create_fixes_geom_index)

ALTER TABLE fixes
    DROP COLUMN IF EXISTS geom;

-- Restore the dropped columns
-- Note: We can't restore the data that was in these columns, they will be NULL/default values
ALTER TABLE fixes
    ADD COLUMN address_type address_type,
    ADD COLUMN aircraft_type_ogn aircraft_type_ogn,
    ADD COLUMN device_address INTEGER,
    ADD COLUMN registration VARCHAR(10);

-- Note: To repopulate these columns from devices table after rollback:
-- UPDATE fixes f SET
--     address_type = d.address_type,
--     aircraft_type_ogn = d.aircraft_type_ogn,
--     device_address = d.address,
--     registration = d.registration
-- FROM devices d WHERE f.device_id = d.id;
