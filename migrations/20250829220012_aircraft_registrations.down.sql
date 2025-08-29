-- =========================================================
-- Reverse the aircraft registrations migration
-- Drop tables in reverse order of creation to handle dependencies
-- =========================================================

-- Drop indexes first
DROP INDEX IF EXISTS aircraft_registrations_serial_idx;
DROP INDEX IF EXISTS aircraft_registrations_transponder_idx;
DROP INDEX IF EXISTS aircraft_registrations_aw_class_idx;
DROP INDEX IF EXISTS aircraft_registrations_status_idx;
DROP INDEX IF EXISTS aircraft_registrations_state_county;
DROP INDEX IF EXISTS aircraft_registrations_eng_mfr_mdl_idx;
DROP INDEX IF EXISTS aircraft_registrations_mfr_mdl_idx;

-- Drop dependent tables first (those with foreign keys)
DROP TABLE IF EXISTS aircraft_other_names;
DROP TABLE IF EXISTS aircraft_registrations;

-- Drop lookup tables
DROP TABLE IF EXISTS status_codes;
DROP TABLE IF EXISTS type_engines;
DROP TABLE IF EXISTS type_aircraft;
DROP TABLE IF EXISTS airworthiness_classes;
DROP TABLE IF EXISTS type_registrations;
DROP TABLE IF EXISTS regions;
DROP TABLE IF EXISTS countries;
DROP TABLE IF EXISTS states;
