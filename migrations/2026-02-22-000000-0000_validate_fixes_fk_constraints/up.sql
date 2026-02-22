-- Validate the two FK constraints on fixes that were created with NOT VALID.
--
-- These were left NOT VALID by the 2026-01-25 deduplicate and 2026-01-30
-- address migration to avoid expensive full-table scans at migration time.
-- The orphaned rows have since been cleaned up by the retention policy,
-- so validation will now succeed.
--
-- This migration runs outside a transaction (see metadata.toml) because
-- VALIDATE CONSTRAINT only takes a SHARE UPDATE EXCLUSIVE lock, allowing
-- concurrent reads and writes while it scans.
ALTER TABLE fixes VALIDATE CONSTRAINT fixes_aircraft_id_fkey;
ALTER TABLE fixes VALIDATE CONSTRAINT fixes_flight_id_fkey;
